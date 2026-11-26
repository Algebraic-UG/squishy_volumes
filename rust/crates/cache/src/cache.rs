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
    available_space: AvailableSpace,

    available_frames: Arc<AtomicUsize>,
    store_thread: Mutex<StoreThread>,
}

const STORE_THREAD_POISONED: &str = "store thread mutex poisoned by an earlier panic";

pub struct Cache {
    state: squishy_volumes_directory_lock::DirectoryLock<CacheState>,
}

pub struct FrameStore {
    sender: FrameSender,
    space_budget: SpaceBudget,
}

impl FrameStore {
    pub fn store_frame(
        &self,
        frame: squishy_volumes_file_frame::Frame,
        stats: Stats,
    ) -> Result<(), Error> {
        self.space_budget.ensure_room_for_a_frame()?;
        self.sender.send(frame, stats)?;
        Ok(())
    }
}

fn ensure_no_frame_missing(
    directory: &std::path::Path,
    frames: &mut [usize],
) -> Result<(), ReadError> {
    frames.sort_unstable();
    match frames
        .iter()
        .enumerate()
        .find(|(expected, frame)| expected != *frame)
    {
        Some((missing, _)) => Err(ReadError::FrameMissing {
            path: frame_path(directory, missing),
        }),
        None => Ok(()),
    }
}

impl CacheState {
    fn new(directory: &std::path::Path, input_bytes_on_disk: u64) -> Result<Self, Error> {
        tracing::info!(?directory, "opening cache");

        let (bytes_on_disk_from_frames, frames) = discover_frames(directory)?;
        let total_bytes_on_disk = input_bytes_on_disk + bytes_on_disk_from_frames;

        let mut frames = frames.into_iter().map(|(i, _)| i).collect::<Vec<_>>();
        ensure_no_frame_missing(directory, &mut frames)?;

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
        truncate_stats(directory, available_frames.load(Ordering::Relaxed))?;

        let store_thread = Mutex::new(StoreThread::idle());

        Ok(Self {
            input_bytes_on_disk,
            total_bytes_on_disk: Arc::new(AtomicU64::new(total_bytes_on_disk)),
            available_space: available_space_on_disk,

            available_frames,
            store_thread,
        })
    }
}

impl Cache {
    pub fn new(
        directory_lock: squishy_volumes_directory_lock::DirectoryLock,
        input_bytes_on_disk: u64,
    ) -> Result<Self, Error> {
        let state = directory_lock
            .try_map(|directory, ()| CacheState::new(directory, input_bytes_on_disk))?;
        Ok(Self { state })
    }

    pub fn directory(&self) -> &std::path::Path {
        self.state.directory()
    }

    pub fn available_frames(&self) -> usize {
        self.state.available_frames.load(Ordering::Acquire)
    }

    pub fn check(&self) -> Result<(), Error> {
        let mut frames = discover_frames(self.state.directory())?
            .1
            .into_iter()
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        ensure_no_frame_missing(self.state.directory(), &mut frames)?;
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

    fn space_budget(&self, limits: SpaceLimits) -> SpaceBudget {
        SpaceBudget::new(
            self.state.directory().to_path_buf(),
            self.state.total_bytes_on_disk.clone(),
            limits,
            self.state.available_space,
        )
    }

    pub fn ensure_free_space(&self, limits: SpaceLimits) -> Result<(), Error> {
        Ok(self.space_budget(limits).ensure_free_space()?)
    }

    pub fn current_bytes_on_disk(&self) -> u64 {
        self.state.total_bytes_on_disk.load(Ordering::Relaxed)
    }

    pub fn start_storing(&self, limits: SpaceLimits) -> FrameStore {
        let space_budget = self.space_budget(limits);
        let sender = self
            .state
            .store_thread
            .lock()
            .expect(STORE_THREAD_POISONED)
            .start(
                self.state.directory().to_path_buf(),
                space_budget.clone(),
                self.state.available_frames.clone(),
            );
        FrameStore {
            sender,
            space_budget,
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
        Ok(
            squishy_volumes_file_frame::Frame::read(frame_path(
                self.state.directory(),
                frame_index,
            ))
            .map_err(Box::new)?,
        )
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
        let truncation = truncate_stats(self.state.directory(), self.available_frames());

        let (bytes_on_disk_from_frames, _frames) = discover_frames(self.state.directory())?;
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
) -> Result<(u64, Vec<(usize, std::path::PathBuf)>), ReadError> {
    let cache_dir = cache_dir.as_ref();
    let list_directory_error = |source| ReadError::ListDirectory {
        path: cache_dir.to_path_buf(),
        source,
    };
    std::fs::read_dir(cache_dir)
        .map_err(list_directory_error)?
        .try_fold((0, Vec::new()), |(mut bytes_on_disk, mut frames), entry| {
            let path = entry.map_err(list_directory_error)?.path();

            if let Some(frame_number) = get_frame_number(&path) {
                bytes_on_disk += frame_size(&path)?;
                frames.push((frame_number, path));
            }

            Ok((bytes_on_disk, frames))
        })
}

fn frame_size(path: &std::path::Path) -> Result<u64, ReadError> {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .map_err(|source| ReadError::FrameSize {
            path: path.to_path_buf(),
            source,
        })
}

pub fn clean_up_frames(
    cache_dir: impl AsRef<std::path::Path>,
    from_frame: usize,
) -> Result<(), CleanupError> {
    let mut frames = discover_frames(cache_dir)
        .map_err(CleanupError::DiscoverFrames)?
        .1;
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

    const UNLIMITED: SpaceLimits = SpaceLimits {
        max_bytes_on_disk: u64::MAX,
        check_free_space: false,
    };

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
        Cache::new(directory_lock, 0).unwrap()
    }

    fn store_empty_frames(cache: &Cache, count: usize) {
        let frame_store = cache.start_storing(UNLIMITED);
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
        let frame_store = cache.start_storing(UNLIMITED);
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

    fn path_of_failed_frame_read(cache: &Cache, frame: usize) -> std::path::PathBuf {
        match cache.fetch_frame(frame) {
            Err(ReadError::Deserialization(error)) => match *error {
                squishy_volumes_file_frame::Error::Read { path, .. } => path,
                error => panic!("unexpected error: {error:?}"),
            },
            Err(error) => panic!("unexpected error: {error:?}"),
            Ok(_) => panic!("frame {frame} was read"),
        }
    }

    #[test]
    fn fetching_a_garbage_frame_names_its_path() {
        let dir = temp_dir("garbage_frame");
        std::fs::write(frame_path(&dir, 0), b"not an alembic file").unwrap();
        let cache = open_cache(&dir);

        assert_eq!(path_of_failed_frame_read(&cache, 0), frame_path(&dir, 0));
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fetching_a_truncated_frame_names_its_path() {
        let dir = temp_dir("truncated_frame");
        let size = squishy_volumes_file_frame::Frame::default()
            .write(frame_path(&dir, 0))
            .unwrap();
        std::fs::OpenOptions::new()
            .write(true)
            .open(frame_path(&dir, 0))
            .unwrap()
            .set_len(size / 2)
            .unwrap();
        let cache = open_cache(&dir);

        assert_eq!(path_of_failed_frame_read(&cache, 0), frame_path(&dir, 0));
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn fetching_a_frame_replaced_by_a_directory_names_its_path() {
        let dir = temp_dir("directory_frame");
        write_empty_frames(&dir, 1);
        let cache = open_cache(&dir);
        std::fs::remove_file(frame_path(&dir, 0)).unwrap();
        std::fs::create_dir(frame_path(&dir, 0)).unwrap();

        assert_eq!(path_of_failed_frame_read(&cache, 0), frame_path(&dir, 0));
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn discovering_frames_in_a_missing_directory_names_it() {
        let dir = temp_dir("missing_directory");
        std::fs::remove_dir(&dir).unwrap();

        assert!(matches!(
            discover_frames(&dir),
            Err(ReadError::ListDirectory { path, .. }) if path == dir
        ));
        assert!(matches!(
            clean_up_frames(&dir, 0),
            Err(CleanupError::DiscoverFrames(ReadError::ListDirectory { path, .. })) if path == dir
        ));
    }

    #[test]
    fn opening_a_cache_with_unreadable_stats_names_them() {
        let dir = temp_dir("unreadable_stats");
        std::fs::create_dir(stats_path(&dir)).unwrap();
        let directory_lock =
            squishy_volumes_directory_lock::DirectoryLock::new(dir.clone()).unwrap();

        assert!(matches!(
            Cache::new(directory_lock, 0),
            Err(Error::Cleanup(CleanupError::ReadStats { path, .. })) if path == stats_path(&dir)
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(unix)]
    fn set_mode(path: &std::path::Path, mode: u32) {
        use std::os::unix::fs::PermissionsExt as _;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn removing_a_frame_from_a_read_only_directory_names_it() {
        let dir = temp_dir("read_only_directory");
        write_empty_frames(&dir, 3);
        set_mode(&dir, 0o555);
        let permissions_enforced = std::fs::write(dir.join("probe"), []).is_err();

        let result = clean_up_frames(&dir, 1);

        set_mode(&dir, 0o755);
        if permissions_enforced {
            assert!(matches!(
                result,
                Err(CleanupError::RemoveFrame { frame: 2, path, .. }) if path == frame_path(&dir, 2)
            ));
            assert_eq!(remaining_frames(&dir), [0, 1, 2]);
        }
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
        let cache = Cache::new(directory_lock, 0).unwrap();

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

    fn cache_with_free_space(
        dir: &std::path::Path,
        input_bytes_on_disk: u64,
        available_space: fn(&std::path::Path) -> Result<u64, WriteError>,
    ) -> Cache {
        let directory_lock =
            squishy_volumes_directory_lock::DirectoryLock::new(dir.to_path_buf()).unwrap();
        let mut cache = Cache::new(directory_lock, input_bytes_on_disk).unwrap();
        cache.state.available_space = available_space;
        cache
    }

    fn limit_of_1000_bytes(check_free_space: bool) -> SpaceLimits {
        SpaceLimits {
            max_bytes_on_disk: 1000,
            check_free_space,
        }
    }

    fn store_one_frame(cache: &Cache, limits: SpaceLimits) -> Result<(), Error> {
        cache.start_storing(limits).store_frame(
            squishy_volumes_file_frame::Frame::default(),
            Stats::default(),
        )
    }

    #[test]
    fn storing_needs_free_space_for_the_remaining_budget() {
        let dir = temp_dir("free_space_short");
        let cache = cache_with_free_space(&dir, 100, |_| Ok(899));

        assert!(matches!(
            store_one_frame(&cache, limit_of_1000_bytes(true)),
            Err(Error::Write(WriteError::InsufficientDiskSpace {
                remaining: 900,
                available: 899
            }))
        ));
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn storing_counts_bytes_already_in_the_cache_towards_the_budget() {
        let dir = temp_dir("free_space_enough");
        let cache = cache_with_free_space(&dir, 100, |_| Ok(900));

        store_one_frame(&cache, limit_of_1000_bytes(true)).unwrap();
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn storing_ignores_free_space_when_the_check_is_off() {
        let dir = temp_dir("free_space_unchecked");
        let cache = cache_with_free_space(&dir, 100, |_| Ok(0));

        store_one_frame(&cache, limit_of_1000_bytes(false)).unwrap();
        drop(cache);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
