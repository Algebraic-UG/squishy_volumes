// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use super::*;

struct LoadedFrame {
    frame: usize,
    state: squishy_volumes_file_frame::IoState,
}

pub struct CachedState<'a> {
    guard: MutexGuard<'a, Option<LoadedFrame>>,
}

impl<'a> std::ops::Deref for CachedState<'a> {
    type Target = squishy_volumes_file_frame::IoState;

    fn deref(&self) -> &Self::Target {
        &self
            .guard
            .as_ref()
            .expect("cached state is never none")
            .state
    }
}

pub struct Cache {
    directory_lock: squishy_volumes_directory_lock::DirectoryLock,

    input_bytes_on_disk: u64,
    total_bytes_on_disk: Arc<AtomicU64>,
    max_bytes_on_disk: Arc<AtomicU64>,

    loaded_frame: Mutex<Option<LoadedFrame>>,

    available_frames: Arc<AtomicUsize>,
    store_thread: Mutex<StoreThread>,
}

impl Cache {
    pub fn new(
        directory_lock: squishy_volumes_directory_lock::DirectoryLock,
        input_bytes_on_disk: u64,
        max_bytes_on_disk: u64,
    ) -> Result<Self, crate::CacheError> {
        let directory = directory_lock.directory();
        tracing::info!(?directory, "opening cache");

        let (bytes_on_disk_from_frames, frames) =
            discover_frames(directory).map_err(CacheReadingError::IoError)?;
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
            tracing::info!("no frames recovered, need to build initial state");
        }

        let store_thread = Mutex::new(StoreThread::new(
            directory.to_path_buf(),
            total_bytes_on_disk.clone(),
            available_frames.clone(),
        ));

        Ok(Self {
            directory_lock,

            input_bytes_on_disk,
            total_bytes_on_disk,
            max_bytes_on_disk,

            loaded_frame: None.into(),
            available_frames,
            store_thread,
        })
    }

    pub fn directory(&self) -> &std::path::Path {
        self.directory_lock.directory()
    }

    pub fn available_frames(&self) -> usize {
        self.available_frames.load(Ordering::Relaxed)
    }

    pub fn check(&self) -> Result<(), CacheError> {
        self.directory_lock.check()?;

        let mut frames = discover_frames(self.directory_lock.directory())
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
        self.store_thread
            .lock()
            .map_err(|_| CacheError::StoreThreadLockPoisoned)?
            .check()?;
        Ok(())
    }

    pub fn set_max_bytes_on_disk(&self, max_bytes_on_disk: u64) {
        self.max_bytes_on_disk
            .store(max_bytes_on_disk, Ordering::Relaxed);
    }

    pub fn current_bytes_on_disk(&self) -> u64 {
        self.total_bytes_on_disk.load(Ordering::Relaxed)
    }

    pub fn store_frame(
        &self,
        state: squishy_volumes_file_frame::IoState,
    ) -> Result<(), CacheError> {
        if self.total_bytes_on_disk.load(Ordering::Relaxed)
            >= self.max_bytes_on_disk.load(Ordering::Relaxed)
        {
            Err(CacheWritingError::ExceedingSpace)?
        }
        self.store_thread
            .lock()
            .map_err(|_| CacheError::StoreThreadLockPoisoned)?
            .store(state)?;
        Ok(())
    }

    pub fn fetch_frame<'a>(&'a self, frame: usize) -> Result<CachedState<'a>, CacheReadingError> {
        let mut loaded_frame = self
            .loaded_frame
            .lock()
            .map_err(|_| CacheReadingError::LoadedFrameLockPoisoned)?;

        if loaded_frame
            .as_ref()
            .is_none_or(|loaded_frame| loaded_frame.frame != frame)
        {
            if frame >= self.available_frames.load(Ordering::Relaxed) {
                return Err(CacheReadingError::FrameNotReady);
            }
            tracing::debug!(frame, "reading frame from disk");
            let state = squishy_volumes_file_frame::IoState::read(frame_path(
                self.directory_lock.directory(),
                frame,
            ))?;
            *loaded_frame = Some(LoadedFrame { frame, state });
        }

        Ok(CachedState {
            guard: loaded_frame,
        })
    }

    pub fn drop_frames(&self, from_frame: usize) -> Result<(), CacheError> {
        let mut store_thread = self
            .store_thread
            .lock()
            .map_err(|_| CacheError::StoreThreadLockPoisoned)?;
        *store_thread = StoreThread::new(
            self.directory_lock.directory().to_path_buf(),
            self.total_bytes_on_disk.clone(),
            self.available_frames.clone(),
        );
        self.available_frames
            .fetch_min(from_frame, Ordering::Relaxed);
        clean_up_frames(self.directory_lock.directory(), from_frame)?;

        let (bytes_on_disk_from_frames, _frames) =
            discover_frames(self.directory_lock.directory()).map_err(CacheReadingError::IoError)?;
        self.total_bytes_on_disk.store(
            self.input_bytes_on_disk + bytes_on_disk_from_frames,
            Ordering::Relaxed,
        );
        Ok(())
    }

    pub fn grid_node_count(&self) -> Result<Option<usize>, CacheError> {
        Ok(self
            .loaded_frame
            .lock()
            .map_err(|_| CacheReadingError::LoadedFrameLockPoisoned)?
            .as_ref()
            .and_then(|loaded_frame| {
                loaded_frame
                    .state
                    .grid_nodes
                    .as_ref()
                    .map(|grid_nodes| grid_nodes.collider_bits.len())
            }))
    }
}

fn discover_frames(
    cache_dir: impl AsRef<std::path::Path>,
) -> Result<(u64, Vec<(usize, std::path::PathBuf)>), std::io::Error> {
    std::fs::read_dir(cache_dir)?.try_fold(
        (0, Vec::new()),
        |(mut bytes_on_disk, mut frames),
         entry|
         -> Result<(u64, Vec<(usize, std::path::PathBuf)>), std::io::Error> {
            let entry = entry?;
            let path = entry.path();

            if let Some(frame_number) = get_frame_number(&path) {
                bytes_on_disk += entry.metadata()?.len();
                frames.push((frame_number, path));
            }

            Ok((bytes_on_disk, frames))
        },
    )
}

pub fn clean_up_frames(
    cache_dir: impl AsRef<std::path::Path>,
    from_frame: usize,
) -> Result<(), CacheCleanupError> {
    for (frame, frame_path) in discover_frames(cache_dir)?.1 {
        if frame >= from_frame {
            std::fs::remove_file(frame_path)?
        }
    }
    Ok(())
}
