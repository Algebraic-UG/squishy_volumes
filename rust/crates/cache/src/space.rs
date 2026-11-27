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
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use super::*;

pub type AvailableSpace = fn(&Path) -> Result<u64, WriteError>;

#[derive(Clone, Copy, Debug)]
pub struct SpaceLimits {
    pub max_bytes_on_disk: u64,
    pub check_free_space: bool,
}

#[derive(Clone)]
pub struct SpaceBudget {
    directory: PathBuf,
    pub total_bytes_on_disk: Arc<AtomicU64>,
    limits: SpaceLimits,
    available_space: AvailableSpace,
}

impl SpaceBudget {
    pub fn new(
        directory: PathBuf,
        total_bytes_on_disk: Arc<AtomicU64>,
        limits: SpaceLimits,
        available_space: AvailableSpace,
    ) -> Self {
        Self {
            directory,
            total_bytes_on_disk,
            limits,
            available_space,
        }
    }

    pub fn ensure_room_for_a_frame(&self) -> Result<(), WriteError> {
        self.ensure_below_limit()?;
        self.ensure_free_space()
    }

    fn ensure_below_limit(&self) -> Result<(), WriteError> {
        if self.total_bytes_on_disk.load(Ordering::Relaxed) >= self.limits.max_bytes_on_disk {
            Err(WriteError::ExceedingSpace)?
        }
        Ok(())
    }

    pub fn ensure_free_space(&self) -> Result<(), WriteError> {
        if !self.limits.check_free_space {
            return Ok(());
        }
        let remaining = self
            .limits
            .max_bytes_on_disk
            .saturating_sub(self.total_bytes_on_disk.load(Ordering::Relaxed));
        let available = (self.available_space)(&self.directory)?;
        if available < remaining {
            Err(WriteError::InsufficientDiskSpace {
                remaining,
                available,
            })?
        }
        Ok(())
    }
}

pub fn available_space_on_disk(directory: &Path) -> Result<u64, WriteError> {
    // windows rejects the root of a subst drive, the canonical path behind it works
    directory
        .canonicalize()
        .and_then(fs4::available_space)
        .map_err(|source| WriteError::QueryFreeSpace {
            path: directory.to_path_buf(),
            source,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_temporary_directory_has_room_for_a_small_cache() {
        let budget = SpaceBudget::new(
            std::env::temp_dir(),
            Arc::default(),
            SpaceLimits {
                max_bytes_on_disk: 1 << 20,
                check_free_space: true,
            },
            available_space_on_disk,
        );

        budget.ensure_free_space().unwrap();
    }

    #[test]
    fn querying_the_free_space_of_a_missing_directory_names_it() {
        let missing = std::env::temp_dir().join("squishy_volumes_missing_cache_directory");

        let error = available_space_on_disk(&missing).unwrap_err();

        assert!(matches!(error, WriteError::QueryFreeSpace { path, .. } if path == missing));
    }
}
