// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Result, bail, ensure};
use std::{
    fs::{File, read_to_string, remove_file},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};
use tracing::error;

fn lock_path<P: AsRef<Path>>(cache_dir: P) -> PathBuf {
    cache_dir.as_ref().join("lock")
}

pub struct CacheLock {
    cache_dir: PathBuf,
    uuid: String,
}

impl CacheLock {
    pub fn new<P: AsRef<Path>>(cache_dir: P, uuid: String) -> Result<Self> {
        let lock_path = lock_path(&cache_dir);
        let result = File::create_new(&lock_path);
        if let Err(e) = &result
            && e.kind() == ErrorKind::AlreadyExists
        {
            bail!(
                "'lock' file exists: {lock_path:?}

Either another simulation is currently using this cache
or the lock is a remnant of a prior crash and you must delete it (sorry)."
            );
        }
        write!(&mut result?, "{uuid}")?;
        Ok(Self {
            cache_dir: cache_dir.as_ref().to_path_buf(),
            uuid,
        })
    }

    pub fn cache_dir(&self) -> &Path {
        self.cache_dir.as_path()
    }

    pub fn check(&self) -> Result<()> {
        ensure!(
            self.uuid == read_to_string(lock_path(&self.cache_dir))?,
            "cache lock's uuid has changed"
        );
        Ok(())
    }
}

impl Drop for CacheLock {
    fn drop(&mut self) {
        if let Err(e) = remove_file(lock_path(&self.cache_dir)) {
            error!("failed to clean up lock file: {e:?}");
        }
    }
}
