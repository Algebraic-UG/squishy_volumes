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

use std::{
    fs::{File, TryLockError},
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("failed to create directory '{directory}'")]
    CreateDirectory {
        directory: PathBuf,
        source: std::io::Error,
    },
    #[error("another simulation is currently using this cache: {0}")]
    AlreadyLocked(PathBuf),
    #[error("failed to open the lock file '{path}'")]
    LockFile {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to lock '{path}'")]
    Lock {
        path: PathBuf,
        source: std::io::Error,
    },
}

fn lock_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join("lock")
}

/// The exclusively locked 'lock' file of a directory.
///
/// The lock is released when the file is closed, which also happens if the process crashes.
/// The file itself is never removed: a process that already opened it could still
/// lock it after removal, while another one creates and locks a new file.
struct LockedFile {
    directory: PathBuf,
    _file: File,
}

impl LockedFile {
    fn new(directory: PathBuf) -> Result<Self, Error> {
        std::fs::create_dir_all(&directory).map_err(|source| {
            tracing::error!(?directory, "Failed to create directory");
            Error::CreateDirectory {
                directory: directory.clone(),
                source,
            }
        })?;

        let path = lock_path(&directory);
        let file = match File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
        {
            Ok(file) => file,
            Err(source) => return Err(Error::LockFile { path, source }),
        };
        match file.try_lock() {
            Ok(()) => Ok(Self {
                directory,
                _file: file,
            }),
            Err(TryLockError::WouldBlock) => {
                tracing::error!(?directory, "Already locked");
                Err(Error::AlreadyLocked(directory))
            }
            Err(TryLockError::Error(source)) => Err(Error::Lock { path, source }),
        }
    }
}

/// Exclusive access to a directory and to the `content` that lives in it.
///
/// Like a [`std::sync::Mutex`], the lock owns what it protects:
/// the content is always dropped before the lock is released.
pub struct DirectoryLock<T = ()> {
    content: Option<T>,
    locked_file: Option<LockedFile>,
}

const INVARIANT: &str = "only taken when consuming the lock";

impl DirectoryLock {
    pub fn new(directory: PathBuf) -> Result<Self, Error> {
        Ok(Self {
            content: Some(()),
            locked_file: Some(LockedFile::new(directory)?),
        })
    }
}

impl<T> DirectoryLock<T> {
    pub fn directory(&self) -> &Path {
        &self.locked_file.as_ref().expect(INVARIANT).directory
    }

    /// Replaces the content while keeping the directory locked.
    ///
    /// If `f` fails, the old content is gone and the lock is released.
    pub fn try_map<U, E>(
        mut self,
        f: impl FnOnce(&Path, T) -> Result<U, E>,
    ) -> Result<DirectoryLock<U>, E> {
        let content = self.content.take().expect(INVARIANT);
        let locked_file = self.locked_file.take().expect(INVARIANT);
        Ok(DirectoryLock {
            content: Some(f(&locked_file.directory, content)?),
            locked_file: Some(locked_file),
        })
    }

    /// Drops the content and runs `clean_up` before the lock is released.
    pub fn release(mut self, clean_up: impl FnOnce(&Path)) {
        drop(self.content.take());
        clean_up(self.directory());
    }
}

impl<T> Deref for DirectoryLock<T> {
    type Target = T;

    fn deref(&self) -> &T {
        self.content.as_ref().expect(INVARIANT)
    }
}

impl<T> DerefMut for DirectoryLock<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.content.as_mut().expect(INVARIANT)
    }
}

impl<T> Drop for DirectoryLock<T> {
    fn drop(&mut self) {
        drop(self.content.take());
        drop(self.locked_file.take());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "squishy_volumes_directory_lock_{name}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn second_lock_fails_while_first_is_held() {
        let dir = temp_dir("exclusive");
        let first = DirectoryLock::new(dir.clone()).unwrap();

        assert!(matches!(
            DirectoryLock::new(dir.clone()),
            Err(Error::AlreadyLocked(_))
        ));

        drop(first);
        drop(DirectoryLock::new(dir.clone()).unwrap());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn stale_lock_file_does_not_block() {
        let dir = temp_dir("stale");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(lock_path(&dir), "left over from a crash").unwrap();

        drop(DirectoryLock::new(dir.clone()).unwrap());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn content_is_dropped_while_locked() {
        struct AssertLockedOnDrop(PathBuf);
        impl Drop for AssertLockedOnDrop {
            fn drop(&mut self) {
                assert!(matches!(
                    DirectoryLock::new(self.0.clone()),
                    Err(Error::AlreadyLocked(_))
                ));
            }
        }

        let dir = temp_dir("drop_order");
        let lock = DirectoryLock::new(dir.clone())
            .unwrap()
            .try_map(|directory, ()| {
                Ok::<_, std::convert::Infallible>(AssertLockedOnDrop(directory.to_path_buf()))
            })
            .unwrap();

        drop(lock);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn release_cleans_up_while_locked() {
        let dir = temp_dir("release");
        let mut cleaned_up = false;
        DirectoryLock::new(dir.clone())
            .unwrap()
            .release(|directory| {
                assert!(matches!(
                    DirectoryLock::new(directory.to_path_buf()),
                    Err(Error::AlreadyLocked(_))
                ));
                cleaned_up = true;
            });

        assert!(cleaned_up);
        drop(DirectoryLock::new(dir.clone()).unwrap());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn file_in_place_of_directory_names_the_directory() {
        let dir = temp_dir("file_in_place");
        std::fs::write(&dir, "not a directory").unwrap();

        let result = DirectoryLock::new(dir.clone());
        assert!(matches!(
            result,
            Err(Error::CreateDirectory { ref directory, .. }) if *directory == dir
        ));
        std::fs::remove_file(dir).unwrap();
    }

    #[test]
    fn directory_in_place_of_lock_file_names_the_lock_file() {
        let dir = temp_dir("directory_in_place");
        std::fs::create_dir_all(lock_path(&dir)).unwrap();

        let result = DirectoryLock::new(dir.clone());
        assert!(matches!(
            result,
            Err(Error::LockFile { ref path, .. }) if *path == lock_path(&dir)
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_map_releases_lock() {
        let dir = temp_dir("failed_map");
        let result = DirectoryLock::new(dir.clone())
            .unwrap()
            .try_map(|_, ()| Err::<(), _>("nope"));

        assert!(matches!(result, Err("nope")));
        drop(DirectoryLock::new(dir.clone()).unwrap());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
