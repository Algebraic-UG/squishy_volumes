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
    path::{Path, PathBuf},
};

#[derive(thiserror::Error, Debug)]
pub enum DirectoryLockingError {
    #[error("Failed to create dir '{directory}'")]
    DirCreation {
        directory: PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error("Another simulation is currently using this cache: {0}")]
    AlreadyLocked(PathBuf),
    #[error("Unknown io error")]
    IoError(#[from] std::io::Error),
}

fn lock_path(directory: impl AsRef<Path>) -> PathBuf {
    directory.as_ref().join("lock")
}

/// Exclusive access to a directory, held through its locked 'lock' file.
///
/// The lock is released when the file is closed, which also happens if the process crashes.
/// The file itself is never removed: a process that already opened it could still
/// lock it after removal, while another one creates and locks a new file.
pub struct DirectoryLock {
    directory: PathBuf,
    _file: File,
}

impl DirectoryLock {
    pub fn new(directory: PathBuf) -> Result<Self, DirectoryLockingError> {
        std::fs::create_dir_all(&directory).map_err(|error| {
            tracing::error!(?directory, "Failed to create directory");
            DirectoryLockingError::DirCreation {
                directory: directory.clone(),
                error,
            }
        })?;

        let file = File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path(&directory))?;
        match file.try_lock() {
            Ok(()) => Ok(Self {
                directory,
                _file: file,
            }),
            Err(TryLockError::WouldBlock) => {
                tracing::error!(?directory, "Already locked");
                Err(DirectoryLockingError::AlreadyLocked(directory))
            }
            Err(TryLockError::Error(error)) => Err(error.into()),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
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
            Err(DirectoryLockingError::AlreadyLocked(_))
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
}
