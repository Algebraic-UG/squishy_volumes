// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fs::{read, read_dir, remove_file},
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
};

use bincode::deserialize;
use tracing::{debug, info};

use squishy_volumes_api::T;

use crate::{state::State, stats::StateStats};

use super::state::attributes::Attribute;

mod store_thread;
pub use store_thread::StoreThread;

mod errors;
pub use errors::*;

pub struct Cache {
    directory: PathBuf,

    input_bytes_on_disk: u64,
    total_bytes_on_disk: Arc<AtomicU64>,
    max_bytes_on_disk: Arc<AtomicU64>,

    loaded_frame: Mutex<Option<(usize, State, StateStats)>>,

    available_frames: Arc<AtomicUsize>,
    store_thread: Mutex<StoreThread>,
}

impl Cache {
    pub fn new(
        directory: PathBuf,
        input_bytes_on_disk: u64,
        max_bytes_on_disk: u64,
    ) -> Result<Self, CacheError> {
        info!("New cache at {directory:?}");
        clean_up_frames(&directory, 0)?;

        Self::load(directory, input_bytes_on_disk, max_bytes_on_disk)
    }

    pub fn load(
        directory: PathBuf,
        input_bytes_on_disk: u64,
        max_bytes_on_disk: u64,
    ) -> Result<Self, CacheError> {
        info!("loading cache at {directory:?}");

        info!("discovering frames in cache");
        let (bytes_on_disk_from_frames, frames) =
            discover_frames(&directory).map_err(CacheReadingError::IoError)?;
        let total_bytes_on_disk = input_bytes_on_disk + bytes_on_disk_from_frames;

        let mut frames = frames.into_iter().map(|(i, _)| i).collect::<Vec<_>>();
        frames.sort();
        if !frames.iter().enumerate().all(|(a, b)| a == *b) {
            Err(CacheReadingError::FrameSequenceBroken)?
        }

        let total_bytes_on_disk = Arc::new(AtomicU64::new(total_bytes_on_disk));
        let max_bytes_on_disk = Arc::new(AtomicU64::new(max_bytes_on_disk));
        let available_frames = Arc::new(AtomicUsize::new(
            frames
                .iter()
                .max()
                .map(|max_frame| max_frame + 1)
                .unwrap_or(0),
        ));
        if available_frames.load(Ordering::Relaxed) == 0 {
            info!("no frames recovered, need to build initial state");
        }

        let store_thread = Mutex::new(StoreThread::new(
            directory.clone(),
            total_bytes_on_disk.clone(),
            available_frames.clone(),
        ));

        Ok(Self {
            directory,

            input_bytes_on_disk,
            total_bytes_on_disk,
            max_bytes_on_disk,

            loaded_frame: None.into(),
            available_frames,
            store_thread,
        })
    }

    pub fn available_frames(&self) -> usize {
        self.available_frames.load(Ordering::Relaxed)
    }

    pub fn check(&self) -> Result<(), CacheError> {
        let mut frames = discover_frames(&self.directory)
            .map_err(CacheReadingError::IoError)?
            .1
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        frames.sort();
        if !frames.iter().enumerate().all(|(a, b)| a == *b) {
            Err(CacheReadingError::FrameSequenceBroken)?
        }
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
        self.total_bytes_on_disk.load(Ordering::Relaxed)
    }

    pub fn store_frame(&self, state: State) -> Result<(), CacheError> {
        if self.total_bytes_on_disk.load(Ordering::Relaxed)
            >= self.max_bytes_on_disk.load(Ordering::Relaxed)
        {
            Err(CacheWritingError::ExceedingSpace)?
        }
        self.store_thread.lock().unwrap().store(state)?;
        Ok(())
    }

    fn load_frame(
        &self,
        loaded_frame: &mut Option<(usize, State, StateStats)>,
        frame: usize,
    ) -> Result<(), CacheReadingError> {
        if loaded_frame
            .as_ref()
            .is_some_and(|(loaded_frame, _, _)| *loaded_frame == frame)
        {
            return Ok(());
        }

        if frame >= self.available_frames.load(Ordering::Relaxed) {
            return Err(CacheReadingError::FrameNotReady);
        }

        debug!(frame, "reading frame from disk");
        let state = deserialize::<State>(
            &read(frame_path(&self.directory, frame)).map_err(CacheReadingError::ReadFrame)?,
        )?;
        let stats = state.stats();

        *loaded_frame = Some((frame, state, stats));

        Ok(())
    }

    pub fn fetch_frame(&self, frame: usize) -> Result<State, CacheError> {
        let mut loaded_frame = self.loaded_frame.lock().unwrap();
        self.load_frame(&mut loaded_frame, frame)?;
        Ok(loaded_frame.as_ref().unwrap().1.clone())
    }

    pub fn available_attributes(&self, frame: usize) -> Result<Vec<Attribute>, CacheError> {
        let mut loaded_frame = self.loaded_frame.lock().unwrap();
        self.load_frame(&mut loaded_frame, frame)?;
        Ok(loaded_frame
            .as_ref()
            .unwrap()
            .1
            .available_attributes()
            .collect())
    }

    pub fn fetch_flat_attribute(
        &self,
        grid_node_size: T,
        frame: usize,
        attribute: Attribute,
    ) -> Result<Vec<T>, CacheError> {
        let mut loaded_frame = self.loaded_frame.lock().unwrap();
        self.load_frame(&mut loaded_frame, frame)?;
        Ok(loaded_frame
            .as_ref()
            .unwrap()
            .1
            .fetch_flat_attribute(grid_node_size, attribute)
            .map_err(CacheReadingError::Attribute)?)
    }

    pub fn drop_frames(&self, from_frame: usize) -> Result<(), CacheError> {
        let mut store_thread = self.store_thread.lock().unwrap();
        *store_thread = StoreThread::new(
            self.directory.clone(),
            self.total_bytes_on_disk.clone(),
            self.available_frames.clone(),
        );
        self.available_frames
            .fetch_min(from_frame, Ordering::Relaxed);
        clean_up_frames(&self.directory, from_frame)?;

        let (bytes_on_disk_from_frames, _frames) =
            discover_frames(&self.directory).map_err(CacheReadingError::IoError)?;
        self.total_bytes_on_disk.store(
            self.input_bytes_on_disk + bytes_on_disk_from_frames,
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

fn discover_frames<P: AsRef<Path>>(
    cache_dir: P,
) -> Result<(u64, Vec<(usize, PathBuf)>), io::Error> {
    Ok(read_dir(cache_dir)?.try_fold(
        (0, Vec::new()),
        |(mut bytes_on_disk, mut frames),
         entry|
         -> Result<(u64, Vec<(usize, PathBuf)>), io::Error> {
            let entry = entry?;
            let path = entry.path();

            if let Some(frame_number) = get_frame_number(&path) {
                bytes_on_disk += entry.metadata()?.len();
                frames.push((frame_number, path));
            }

            Ok((bytes_on_disk, frames))
        },
    )?)
}

fn clean_up_frames<P: AsRef<Path>>(
    cache_dir: P,
    from_frame: usize,
) -> Result<(), CacheCleanupError> {
    for (frame, frame_path) in discover_frames(cache_dir)?.1 {
        if frame >= from_frame {
            remove_file(frame_path)?
        }
    }
    Ok(())
}
