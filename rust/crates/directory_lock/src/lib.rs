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

use std::io::Write as _;

#[derive(thiserror::Error, Debug)]
pub enum DirectoryLockingError {
    #[error("Failed to create dir '{directory}'")]
    DirCreation {
        directory: std::path::PathBuf,
        #[source]
        error: std::io::Error,
    },
    #[error(
        "'lock' file exists: {0}

Either another simulation is currently using this cache
or the lock is a remnant of a prior crash and you must delete it (sorry)."
    )]
    AlreadyLocked(std::path::PathBuf),
    #[error("The cache lock's UUID has changed from {was} to {is}.")]
    UuidChanged { was: String, is: String },
    #[error("Unknown io error")]
    IoError(#[from] std::io::Error),
}

fn lock_path(directory: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    directory.as_ref().join("lock")
}

pub struct DirectoryLock {
    directory: std::path::PathBuf,
    uuid: String,
}

impl DirectoryLock {
    pub fn new(directory: std::path::PathBuf, uuid: String) -> Result<Self, DirectoryLockingError> {
        std::fs::create_dir_all(&directory).map_err(|error| {
            tracing::error!(?directory, "Failed to create directory");
            DirectoryLockingError::DirCreation {
                directory: directory.clone(),
                error,
            }
        })?;

        let lock_path = lock_path(&directory);
        let result = std::fs::File::create_new(&lock_path);
        if let Err(e) = &result
            && e.kind() == std::io::ErrorKind::AlreadyExists
        {
            tracing::error!(?directory, "Already locked");
            return Err(DirectoryLockingError::AlreadyLocked(directory));
        }
        write!(&mut result?, "{uuid}")?;
        Ok(Self { directory, uuid })
    }

    pub fn directory(&self) -> &std::path::Path {
        self.directory.as_path()
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    pub fn check(&self) -> Result<(), DirectoryLockingError> {
        let uuid = std::fs::read_to_string(lock_path(&self.directory))?;
        if self.uuid != uuid {
            tracing::error!(found = uuid, expected = self.uuid, directory = ?self.directory, "UUID changed");
            return Err(DirectoryLockingError::UuidChanged {
                was: self.uuid.clone(),
                is: uuid,
            });
        }
        Ok(())
    }
}

impl Drop for DirectoryLock {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_file(lock_path(&self.directory)) {
            tracing::error!("failed to clean up lock file: {e:?}");
        }
    }
}
