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
    Arc, Mutex,
    atomic::{AtomicU64, AtomicUsize, Ordering},
};

use super::*;

struct CacheState {
    input_bytes_on_disk: u64,
    total_bytes_on_disk: Arc<AtomicU64>,
    max_bytes_on_disk: Arc<AtomicU64>,

    available_frames: Arc<AtomicUsize>,
    store_thread: Mutex<StoreThread>,
}

const STORE_THREAD_POISONED: &str = "store thread mutex poisoned by an earlier panic";

pub struct Cache {
    state: squishy_volumes_directory_lock::DirectoryLock<CacheState>,
}

pub struct FrameStore {
    sender: FrameSender,
    total_bytes_on_disk: Arc<AtomicU64>,
    max_bytes_on_disk: Arc<AtomicU64>,
}

#[allow(clippy::result_large_err)]
impl FrameStore {
    pub fn store_frame(
        &self,
        frame: squishy_volumes_file_frame::Frame,
        stats: Stats,
    ) -> Result<(), Error> {
        if self.total_bytes_on_disk.load(Ordering::Relaxed)
            >= self.max_bytes_on_disk.load(Ordering::Relaxed)
        {
            Err(WriteError::ExceedingSpace)?
        }
        self.sender.send(frame, stats)?;
        Ok(())
    }
}

#[allow(clippy::result_large_err)]
impl CacheState {
    fn new(
        directory: &std::path::Path,
        input_bytes_on_disk: u64,
        max_bytes_on_disk: u64,
    ) -> Result<Self, Error> {
        tracing::info!(?directory, "opening cache");

        let (bytes_on_disk_from_frames, frames) =
            discover_frames(directory).map_err(ReadError::Io)?;
        let total_bytes_on_disk = input_bytes_on_disk + bytes_on_disk_from_frames;

        let mut frames = frames.into_iter().map(|(i, _)| i).collect::<Vec<_>>();
        frames.sort();
        if !frames.iter().enumerate().all(|(a, b)| a == *b) {
            Err(ReadError::FrameSequenceBroken)?
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
            .map_err(CleanupError::Io)?;

        let store_thread = Mutex::new(StoreThread::idle());

        Ok(Self {
            input_bytes_on_disk,
            total_bytes_on_disk,
            max_bytes_on_disk,

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
    ) -> Result<Self, Error> {
        let state = directory_lock.try_map(|directory, ()| {
            CacheState::new(directory, input_bytes_on_disk, max_bytes_on_disk)
        })?;
        Ok(Self { state })
    }

    pub fn directory(&self) -> &std::path::Path {
        self.state.directory()
    }

    pub fn available_frames(&self) -> usize {
        self.state.available_frames.load(Ordering::Acquire)
    }

    pub fn check(&self) -> Result<(), Error> {
        let mut frames = discover_frames(self.state.directory())
            .map_err(ReadError::Io)?
            .1
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        frames.sort();
        if !frames.iter().enumerate().all(|(a, b)| a == *b) {
            Err(ReadError::FrameSequenceBroken)?
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
            .expect(STORE_THREAD_POISONED)
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

    pub fn start_storing(&self) -> FrameStore {
        let sender = self
            .state
            .store_thread
            .lock()
            .expect(STORE_THREAD_POISONED)
            .start(
                self.state.directory().to_path_buf(),
                self.state.total_bytes_on_disk.clone(),
                self.state.available_frames.clone(),
            );
        FrameStore {
            sender,
            total_bytes_on_disk: self.state.total_bytes_on_disk.clone(),
            max_bytes_on_disk: self.state.max_bytes_on_disk.clone(),
        }
    }

    pub fn storing(&self) -> bool {
        self.state
            .store_thread
            .lock()
            .expect(STORE_THREAD_POISONED)
            .running()
    }

    pub fn fetch_frame(
        &self,
        frame_index: usize,
    ) -> Result<squishy_volumes_file_frame::Frame, ReadError> {
        if frame_index >= self.state.available_frames.load(Ordering::Acquire) {
            return Err(ReadError::FrameNotReady);
        }
        tracing::debug!(frame_index, "reading frame from disk");
        Ok(squishy_volumes_file_frame::Frame::read(frame_path(
            self.state.directory(),
            frame_index,
        ))?)
    }

    pub fn drop_frames(&self, from_frame: usize) -> Result<(), Error> {
        let mut store_thread = self.state.store_thread.lock().expect(STORE_THREAD_POISONED);
        store_thread.stop();
        self.state
            .available_frames
            .fetch_min(from_frame, Ordering::Relaxed);
        let cleanup = clean_up_frames(self.state.directory(), from_frame);
        if let Err(CleanupError::RemoveFrame { frame, .. }) = &cleanup {
            self.state
                .available_frames
                .store(frame + 1, Ordering::Relaxed);
        }
        let truncation = truncate_stats(self.state.directory(), self.available_frames())
            .map_err(CleanupError::Io);

        let (bytes_on_disk_from_frames, _frames) =
            discover_frames(self.state.directory()).map_err(ReadError::Io)?;
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
) -> Result<(), CleanupError> {
    let mut frames = discover_frames(cache_dir)?.1;
    frames.retain(|(frame, _)| *frame >= from_frame);
    frames.sort_unstable_by_key(|(frame, _)| std::cmp::Reverse(*frame));
    for (frame, path) in frames {
        std::fs::remove_file(&path).map_err(|source| CleanupError::RemoveFrame {
            frame,
            path,
            source,
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

    fn stats_rows(dir: &std::path::Path) -> usize {
        std::fs::read_to_string(stats_path(dir))
            .map(|content| content.lines().count().saturating_sub(1))
            .unwrap_or(0)
    }

    fn open_cache(dir: &std::path::Path) -> Cache {
        let directory_lock =
            squishy_volumes_directory_lock::DirectoryLock::new(dir.to_path_buf()).unwrap();
        Cache::new(directory_lock, 0, u64::MAX).unwrap()
    }

    fn store_empty_frames(cache: &Cache, count: usize) {
        let frame_store = cache.start_storing();
        for _ in 0..count {
            frame_store
                .store_frame(
                    squishy_volumes_file_frame::Frame::default(),
                    Stats::default(),
                )
                .unwrap();
        }
    }

    #[test]
    fn drop_frames_keeps_only_written_frames_below_cut() {
        let dir = temp_dir("drop_frames_queued");
        let cut = 4;
        let cache = open_cache(&dir);
        store_empty_frames(&cache, 6);

        cache.drop_frames(cut).unwrap();

        let kept = cache.available_frames();
        assert!(kept <= cut);
        assert_eq!(remaining_frames(&dir), (0..kept).collect::<Vec<_>>());
        assert_eq!(stats_rows(&dir), kept);
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn dropping_cache_leaves_consistent_frames_and_stats() {
        let dir = temp_dir("drop_cache_queued");
        let stored = 6;
        let cache = open_cache(&dir);
        store_empty_frames(&cache, stored);

        drop(cache);

        let written = remaining_frames(&dir);
        assert!(written.len() + QUEUED_FRAMES + 1 >= stored);
        assert_eq!(written, (0..written.len()).collect::<Vec<_>>());
        assert_eq!(stats_rows(&dir), written.len());
        assert_eq!(open_cache(&dir).available_frames(), written.len());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn storing_until_queued_frames_are_written() {
        let dir = temp_dir("storing_until_written");
        let stored = 6;
        let cache = open_cache(&dir);
        store_empty_frames(&cache, stored);

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while cache.storing() {
            assert!(std::time::Instant::now() < deadline, "still storing");
            std::thread::yield_now();
        }

        assert_eq!(cache.available_frames(), stored);
        cache.check().unwrap();
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn storing_while_frame_store_is_alive() {
        let dir = temp_dir("storing_while_alive");
        let cache = open_cache(&dir);
        let frame_store = cache.start_storing();
        assert!(cache.storing());

        drop(frame_store);
        cache.drop_frames(0).unwrap();
        assert!(!cache.storing());
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
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
            Error::Cleanup(CleanupError::RemoveFrame { frame: 5, .. })
        ));
        assert_eq!(cache.available_frames(), 6);
        assert_eq!(remaining_frames(&dir), [0, 1, 2, 3, 4, 5]);

        drop(held_frame);
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
