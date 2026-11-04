// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use super::*;

struct LoadedFrame {
    frame_index: usize,
    frame: squishy_volumes_file_frame::Frame,
}

pub struct CachedState<'a> {
    guard: MutexGuard<'a, Option<LoadedFrame>>,
}

impl<'a> std::ops::Deref for CachedState<'a> {
    type Target = squishy_volumes_file_frame::Frame;

    fn deref(&self) -> &Self::Target {
        &self
            .guard
            .as_ref()
            .expect("cached state is never none")
            .frame
    }
}

struct CacheState {
    input_bytes_on_disk: u64,
    total_bytes_on_disk: Arc<AtomicU64>,
    max_bytes_on_disk: Arc<AtomicU64>,

    loaded_frame: Mutex<Option<LoadedFrame>>,

    available_frames: Arc<AtomicUsize>,
    store_thread: Mutex<StoreThread>,
}

pub struct Cache {
    state: squishy_volumes_directory_lock::DirectoryLock<CacheState>,
}

#[allow(clippy::result_large_err)]
impl CacheState {
    fn new(
        directory: &std::path::Path,
        input_bytes_on_disk: u64,
        max_bytes_on_disk: u64,
    ) -> Result<Self, crate::CacheError> {
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
        truncate_stats(directory, available_frames.load(Ordering::Relaxed))
            .map_err(CacheCleanupError::IoError)?;

        let store_thread = Mutex::new(StoreThread::new(
            directory.to_path_buf(),
            total_bytes_on_disk.clone(),
            available_frames.clone(),
        ));

        Ok(Self {
            input_bytes_on_disk,
            total_bytes_on_disk,
            max_bytes_on_disk,

            loaded_frame: None.into(),
            available_frames,
            store_thread,
        })
    }
}

#[allow(clippy::result_large_err)]
impl Cache {
    pub fn new(
        directory_lock: squishy_volumes_directory_lock::DirectoryLock,
        input_bytes_on_disk: u64,
        max_bytes_on_disk: u64,
    ) -> Result<Self, crate::CacheError> {
        let state = directory_lock.try_map(|directory, ()| {
            CacheState::new(directory, input_bytes_on_disk, max_bytes_on_disk)
        })?;
        Ok(Self { state })
    }

    pub fn directory(&self) -> &std::path::Path {
        self.state.directory()
    }

    pub fn available_frames(&self) -> usize {
        self.state.available_frames.load(Ordering::Relaxed)
    }

    pub fn check(&self) -> Result<(), CacheError> {
        let mut frames = discover_frames(self.state.directory())
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
            self.state.available_frames.load(Ordering::Relaxed)
                == frames
                    .iter()
                    .max()
                    .map(|frame_max| frame_max + 1)
                    .unwrap_or(0),
            "frames are missing at the end"
        );
        */
        self.state
            .store_thread
            .lock()
            .map_err(|_| CacheError::StoreThreadLockPoisoned)?
            .check()?;
        Ok(())
    }

    pub fn set_max_bytes_on_disk(&self, max_bytes_on_disk: u64) {
        self.state
            .max_bytes_on_disk
            .store(max_bytes_on_disk, Ordering::Relaxed);
    }

    pub fn current_bytes_on_disk(&self) -> u64 {
        self.state.total_bytes_on_disk.load(Ordering::Relaxed)
    }

    pub fn store_frame(
        &self,
        frame: squishy_volumes_file_frame::Frame,
        stats: Stats,
    ) -> Result<(), CacheError> {
        if self.state.total_bytes_on_disk.load(Ordering::Relaxed)
            >= self.state.max_bytes_on_disk.load(Ordering::Relaxed)
        {
            Err(CacheWritingError::ExceedingSpace)?
        }
        self.state
            .store_thread
            .lock()
            .map_err(|_| CacheError::StoreThreadLockPoisoned)?
            .store(frame, stats)?;
        Ok(())
    }

    pub fn fetch_frame<'a>(
        &'a self,
        frame_index: usize,
    ) -> Result<CachedState<'a>, CacheReadingError> {
        let mut loaded_frame = self
            .state
            .loaded_frame
            .lock()
            .map_err(|_| CacheReadingError::LoadedFrameLockPoisoned)?;

        if loaded_frame
            .as_ref()
            .is_none_or(|loaded_frame| loaded_frame.frame_index != frame_index)
        {
            if frame_index >= self.state.available_frames.load(Ordering::Relaxed) {
                return Err(CacheReadingError::FrameNotReady);
            }
            tracing::debug!(frame_index, "reading frame from disk");
            let frame = squishy_volumes_file_frame::Frame::read(frame_path(
                self.state.directory(),
                frame_index,
            ))?;
            *loaded_frame = Some(LoadedFrame { frame_index, frame });
        }

        Ok(CachedState {
            guard: loaded_frame,
        })
    }

    pub fn drop_frames(&self, from_frame: usize) -> Result<(), CacheError> {
        let mut store_thread = self
            .state
            .store_thread
            .lock()
            .map_err(|_| CacheError::StoreThreadLockPoisoned)?;
        // dropping the old thread finishes its pending writes before we clean up
        *store_thread = StoreThread::new(
            self.state.directory().to_path_buf(),
            self.state.total_bytes_on_disk.clone(),
            self.state.available_frames.clone(),
        );
        self.state
            .available_frames
            .fetch_min(from_frame, Ordering::Relaxed);
        let cleanup = clean_up_frames(self.state.directory(), from_frame);
        if let Err(CacheCleanupError::RemoveFrame { frame, .. }) = &cleanup {
            self.state
                .available_frames
                .store(frame + 1, Ordering::Relaxed);
        }
        let truncation = truncate_stats(self.state.directory(), self.available_frames())
            .map_err(CacheCleanupError::IoError);

        let (bytes_on_disk_from_frames, _frames) =
            discover_frames(self.state.directory()).map_err(CacheReadingError::IoError)?;
        self.state.total_bytes_on_disk.store(
            self.state.input_bytes_on_disk + bytes_on_disk_from_frames,
            Ordering::Relaxed,
        );
        cleanup?;
        truncation?;
        Ok(())
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

/// Removes all frames from `from_frame` onwards, starting with the last one.
///
/// See <https://github.com/Algebraic-UG/squishy_volumes/issues/422>
pub fn clean_up_frames(
    cache_dir: impl AsRef<std::path::Path>,
    from_frame: usize,
) -> Result<(), CacheCleanupError> {
    let mut frames = discover_frames(cache_dir)?.1;
    frames.retain(|(frame, _)| *frame >= from_frame);
    frames.sort_unstable_by_key(|(frame, _)| std::cmp::Reverse(*frame));
    for (frame, path) in frames {
        std::fs::remove_file(&path).map_err(|error| CacheCleanupError::RemoveFrame {
            frame,
            path,
            error,
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "squishy_volumes_cache_test_{name}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_empty_frames(dir: &std::path::Path, count: usize) {
        for frame in 0..count {
            std::fs::write(frame_path(dir, frame), []).unwrap();
        }
    }

    fn remaining_frames(dir: &std::path::Path) -> Vec<usize> {
        let mut frames = discover_frames(dir)
            .unwrap()
            .1
            .into_iter()
            .map(|(frame, _)| frame)
            .collect::<Vec<_>>();
        frames.sort();
        frames
    }

    #[test]
    fn clean_up_removes_frames_from_given_one() {
        let dir = temp_dir("clean_up");
        write_empty_frames(&dir, 5);

        clean_up_frames(&dir, 2).unwrap();

        assert_eq!(remaining_frames(&dir), [0, 1]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    /// Holds a frame open the way Blender does, see
    /// <https://github.com/Algebraic-UG/squishy_volumes/issues/422>
    #[cfg(windows)]
    #[test]
    fn drop_frames_stops_at_frame_held_open() {
        use std::os::windows::fs::OpenOptionsExt as _;
        const FILE_SHARE_READ: u32 = 0x1;
        const FILE_SHARE_WRITE: u32 = 0x2;

        let dir = temp_dir("held_open");
        write_empty_frames(&dir, 8);
        let directory_lock =
            squishy_volumes_directory_lock::DirectoryLock::new(dir.clone()).unwrap();
        let cache = Cache::new(directory_lock, 0, u64::MAX).unwrap();

        let held_frame = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(frame_path(&dir, 5))
            .unwrap();

        let error = cache.drop_frames(2).unwrap_err();

        assert!(matches!(
            error,
            CacheError::Cleanup(CacheCleanupError::RemoveFrame { frame: 5, .. })
        ));
        assert_eq!(cache.available_frames(), 6);
        assert_eq!(remaining_frames(&dir), [0, 1, 2, 3, 4, 5]);

        drop(held_frame);
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
