// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::api::{SerializedSetup, Setup, StateStats};
use anyhow::{Context, Result, ensure};
use bincode::deserialize;
use lock::CacheLock;
use serde_json::{Value, from_reader, from_value, to_writer_pretty};
use squishy_volumes_api::T;
use std::{
    fs::{File, canonicalize, create_dir_all, metadata, read, read_dir, remove_file},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};
use tracing::{debug, info, warn};

use super::{State, state::attributes::Attribute};

mod lock;
mod store_thread;

pub use store_thread::StoreThread;

pub struct Cache {
    pub setup: Arc<Setup>,

    bytes_on_disk: Arc<AtomicU64>,
    max_bytes_on_disk: Arc<AtomicU64>,

    loaded_frame: Mutex<Option<(usize, State, StateStats)>>,
    cache_lock: CacheLock,
    available_frames: Arc<AtomicUsize>,
    store_thread: Mutex<StoreThread>,
}

impl Cache {
    pub fn new(
        uuid: String,
        setup: Value,
        cache_dir: PathBuf,
        max_bytes_on_disk: u64,
    ) -> Result<Self> {
        info!("creating new cache: {cache_dir:?}");
        create_dir_all(&cache_dir).context("directory creation")?;

        info!("locking it");
        let cache_lock = CacheLock::new(&cache_dir, uuid)?;

        info!("parsing setup");
        let parsed_setup = Arc::new(from_value::<SerializedSetup>(setup.clone())?.try_into()?);

        info!("cleaning up old frames");
        clean_up_frames(&cache_dir, 0).context("clean up old frames")?;

        info!("write setup to disk");
        let mut writer =
            BufWriter::new(File::create(setup_path(&cache_dir)).context("setup file creation")?);
        to_writer_pretty(&mut writer, &setup).context("setup file writing")?;
        writer.flush().context("setup file flushing ")?;

        let bytes_on_disk = Arc::new(AtomicU64::new(writer.into_inner()?.metadata()?.len()));
        let max_bytes_on_disk = Arc::new(AtomicU64::new(max_bytes_on_disk));
        let available_frames = Arc::new(AtomicUsize::new(0));

        let store_thread = Mutex::new(StoreThread::new(
            cache_dir,
            bytes_on_disk.clone(),
            available_frames.clone(),
        ));

        Ok(Self {
            setup: parsed_setup,

            bytes_on_disk,
            max_bytes_on_disk,

            loaded_frame: None.into(),
            cache_lock,
            available_frames,
            store_thread,
        })
    }

    pub fn load(uuid: String, cache_dir: PathBuf, max_bytes_on_disk: u64) -> Result<Self> {
        info!("loading old cache: {:?}", canonicalize(&cache_dir));

        info!("locking it");
        let cache_lock = CacheLock::new(&cache_dir, uuid)?;

        info!("reading setup from disk");
        let setup = File::open(setup_path(&cache_dir)).context("opening setup file")?;
        let mut bytes_on_disk = setup.metadata()?.len();
        let setup: SerializedSetup = from_reader(setup).context("reading setup file")?;

        info!("parsing setup");
        let setup = Arc::new(setup.try_into().context("parsing setup")?);

        info!("discovering frames in cache");
        let (bytes_on_disk_from_frames, frames) =
            discover_frames(&cache_dir).context("discorvering frames")?;
        bytes_on_disk += bytes_on_disk_from_frames;

        let mut frames = frames.into_iter().map(|(i, _)| i).collect::<Vec<_>>();
        frames.sort();
        ensure!(
            frames.iter().enumerate().all(|(a, b)| a == *b),
            "frames are missing"
        );

        let bytes_on_disk = Arc::new(AtomicU64::new(bytes_on_disk));
        let max_bytes_on_disk = Arc::new(AtomicU64::new(max_bytes_on_disk));
        let available_frames = Arc::new(AtomicUsize::new(
            frames
                .iter()
                .max()
                .map(|max_frame| max_frame + 1)
                .unwrap_or(0),
        ));
        if available_frames.load(Ordering::Relaxed) == 0 {
            warn!("no frames recovered, need to build initial state");
        }

        let store_thread = Mutex::new(StoreThread::new(
            cache_dir,
            bytes_on_disk.clone(),
            available_frames.clone(),
        ));

        Ok(Self {
            setup,

            bytes_on_disk,
            max_bytes_on_disk,

            loaded_frame: None.into(),
            cache_lock,
            available_frames,
            store_thread,
        })
    }

    pub fn available_frames(&self) -> usize {
        self.available_frames.load(Ordering::Relaxed)
    }

    pub fn check(&self) -> Result<()> {
        self.cache_lock.check()?;
        metadata(setup_path(self.cache_lock.cache_dir())).context("meta data of setup")?;
        let mut frames = discover_frames(self.cache_lock.cache_dir())
            .context("discovering frames")?
            .1
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        frames.sort();
        ensure!(
            frames.iter().enumerate().all(|(a, b)| a == *b),
            "frames are missing"
        );
        /* TODO: this check can fail.. we don't stop the store thread
        ensure!(
            self.available_frames.load(Ordering::Relaxed)
                == frames
                    .iter()
                    .max()
                    .map(|frame_max| frame_max + 1)
                    .unwrap_or(0),
            "frames are missing at the end"
        );
        */
        self.store_thread.lock().unwrap().check()?;
        Ok(())
    }

    pub fn set_max_bytes_on_disk(&self, max_bytes_on_disk: u64) {
        self.max_bytes_on_disk
            .store(max_bytes_on_disk, Ordering::Relaxed);
    }

    pub fn current_bytes_on_disk(&self) -> u64 {
        self.bytes_on_disk.load(Ordering::Relaxed)
    }

    pub fn store_frame(&self, state: State) -> Result<()> {
        ensure!(
            self.bytes_on_disk.load(Ordering::Relaxed)
                < self.max_bytes_on_disk.load(Ordering::Relaxed),
            "Exceeding allowed disk space"
        );
        self.store_thread.lock().unwrap().store(state)
    }

    fn load_frame(
        &self,
        loaded_frame: &mut Option<(usize, State, StateStats)>,
        frame: usize,
    ) -> Result<()> {
        if loaded_frame
            .as_ref()
            .is_some_and(|(loaded_frame, _, _)| *loaded_frame == frame)
        {
            return Ok(());
        }

        ensure!(
            frame < self.available_frames.load(Ordering::Relaxed),
            "frame not computed yet"
        );

        debug!(frame, "reading frame from disk");
        let state = deserialize::<State>(
            &read(frame_path(self.cache_lock.cache_dir(), frame)).context("read frame")?,
        )
        .context("decoding frame")?;
        let stats = state.stats();

        *loaded_frame = Some((frame, state, stats));

        Ok(())
    }

    pub fn fetch_frame(&self, frame: usize) -> Result<State> {
        let mut loaded_frame = self.loaded_frame.lock().unwrap();
        self.load_frame(&mut loaded_frame, frame)?;
        Ok(loaded_frame.as_ref().unwrap().1.clone())
    }

    pub fn available_attributes(&self, frame: usize) -> Result<Vec<Attribute>> {
        let mut loaded_frame = self.loaded_frame.lock().unwrap();
        self.load_frame(&mut loaded_frame, frame)?;
        Ok(loaded_frame
            .as_ref()
            .unwrap()
            .1
            .available_attributes()
            .collect())
    }

    pub fn fetch_flat_attribute(&self, frame: usize, attribute: Attribute) -> Result<Vec<T>> {
        let mut loaded_frame = self.loaded_frame.lock().unwrap();
        self.load_frame(&mut loaded_frame, frame)?;
        loaded_frame
            .as_ref()
            .unwrap()
            .1
            .fetch_flat_attribute(self.setup.settings.grid_node_size, attribute)
    }

    pub fn drop_frames(&self, from_frame: usize) -> Result<()> {
        let mut store_thread = self.store_thread.lock().unwrap();
        *store_thread = StoreThread::new(
            self.cache_lock.cache_dir().to_path_buf(),
            self.bytes_on_disk.clone(),
            self.available_frames.clone(),
        );
        self.available_frames
            .fetch_min(from_frame, Ordering::Relaxed);
        clean_up_frames(self.cache_lock.cache_dir(), from_frame)?;

        self.bytes_on_disk.store(
            metadata(setup_path(self.cache_lock.cache_dir()))
                .context("meta data of setup")?
                .len()
                + discover_frames(self.cache_lock.cache_dir())?.0,
            Ordering::Relaxed,
        );
        Ok(())
    }

    pub fn loaded_state_stats(&self) -> Option<StateStats> {
        self.loaded_frame
            .lock()
            .unwrap()
            .as_ref()
            .map(|(_, _, stats)| stats.clone())
    }
}

fn setup_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
    cache_dir.as_ref().join("setup.json")
}

fn frame_path<P: AsRef<Path>>(cache_dir: P, frame: usize) -> PathBuf {
    cache_dir.as_ref().join(format!("frame_{frame:05}.bin"))
}

fn get_frame_number<P: AsRef<Path>>(frame_path: P) -> Option<usize> {
    if !frame_path.as_ref().is_file() {
        return None;
    }
    let file_name = frame_path.as_ref().file_name()?.to_str()?;
    if !file_name.starts_with("frame_") || !file_name.ends_with(".bin") {
        return None;
    }
    file_name
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<usize>()
        .ok()
}

fn discover_frames<P: AsRef<Path>>(cache_dir: P) -> Result<(u64, Vec<(usize, PathBuf)>)> {
    Ok(read_dir(cache_dir)
        .context("reading cache directory")?
        .try_fold(
            (0, Vec::new()),
            |(mut bytes_on_disk, mut frames), entry| -> Result<(u64, Vec<(usize, PathBuf)>)> {
                let entry = entry.context("getting_dir entry")?;
                let path = entry.path();

                if let Some(frame_number) = get_frame_number(&path) {
                    bytes_on_disk += entry.metadata().context("reading file size")?.len();
                    frames.push((frame_number, path));
                }

                Ok((bytes_on_disk, frames))
            },
        )?)
}

fn clean_up_frames<P: AsRef<Path>>(cache_dir: P, from_frame: usize) -> Result<()> {
    for (frame, frame_path) in discover_frames(cache_dir)?.1 {
        if frame >= from_frame {
            remove_file(frame_path)?
        }
    }
    Ok(())
}
