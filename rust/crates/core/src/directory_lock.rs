// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::io;
use std::{
    fs::{File, create_dir_all, read_to_string, remove_file},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};
use thiserror::Error;
use tracing::{error, info};

#[derive(Error, Debug)]
pub enum DirectoryLockingError {
    #[error("Failed to create dir '{directory}': {error:?}")]
    DirCreation {
        directory: PathBuf,
        error: io::Error,
    },
    #[error(
        "'lock' file exists: {0}

Either another simulation is currently using this cache
or the lock is a remnant of a prior crash and you must delete it (sorry)."
    )]
    AlreadyLocked(PathBuf),
    #[error("The cache lock's UUID has changed.")]
    UuidChanged,
    #[error("Unknown io error: {0}")]
    IoError(#[from] std::io::Error),
}

fn lock_path<P: AsRef<Path>>(directory: P) -> PathBuf {
    directory.as_ref().join("lock")
}

pub struct DirectoryLock {
    directory: PathBuf,
    uuid: String,
}

impl DirectoryLock {
    pub fn new(directory: PathBuf, uuid: String) -> Result<Self, DirectoryLockingError> {
        info!("creating new directory: {directory:?}");
        create_dir_all(&directory).map_err(|error| DirectoryLockingError::DirCreation {
            directory: directory.clone(),
            error,
        })?;

        info!("Locking directory");
        let lock_path = lock_path(&directory);
        let result = File::create_new(&lock_path);
        if let Err(e) = &result
            && e.kind() == ErrorKind::AlreadyExists
        {
            return Err(DirectoryLockingError::AlreadyLocked(directory));
        }
        write!(&mut result?, "{uuid}")?;
        Ok(Self { directory, uuid })
    }

    pub fn directory(&self) -> &Path {
        self.directory.as_path()
    }

    pub fn uuid(&self) -> &str {
        &self.uuid
    }

    pub fn check(&self) -> Result<(), DirectoryLockingError> {
        if self.uuid == read_to_string(lock_path(&self.directory))? {
            return Err(DirectoryLockingError::UuidChanged);
        }
        Ok(())
    }
}

impl Drop for DirectoryLock {
    fn drop(&mut self) {
        if let Err(e) = remove_file(lock_path(&self.directory)) {
            error!("failed to clean up lock file: {e:?}");
        }
    }
}
