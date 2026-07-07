// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::io::Write as _;

#[derive(thiserror::Error, Debug)]
pub enum DirectoryLockingError {
    #[error("Failed to create dir '{directory}': {error:?}")]
    DirCreation {
        directory: std::path::PathBuf,
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
    #[error("Unknown io error: {0}")]
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
