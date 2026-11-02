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

// The structure is simple, there are a few things that should remain stable across versions
// followed by a bunch of things that are almost completely handled by serde.
// So those typically break between versions, but there might be migration paths later.
//
// =============================================================================
// Stable:
// =============================================================================
//
// 32 Magic bytes: binary some string
// 64 Version Bytes: any version string like "10.1337.42-alpha" should fit this
//
// =============================================================================
// Unstable:
// =============================================================================
//
// depends on the actual use, at this time, it's either the input file or io states
//

pub const MAGIC_LEN: usize = 32;
pub const VERSION_LEN: usize = 64;
pub const DATA_OFFSET: usize = MAGIC_LEN + VERSION_LEN;

build_info::build_info!(fn build_info);

pub fn version_string() -> String {
    build_info().crate_info.version.to_string()
}

fn version_bytes() -> [u8; VERSION_LEN] {
    let version_string = version_string();
    let bytes = version_string.as_bytes();
    assert!(bytes.len() <= VERSION_LEN, "Version string too long");
    std::array::from_fn(|i| if i < bytes.len() { bytes[i] } else { 0 })
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to write magic bytes")]
    WriteMagic(#[source] std::io::Error),
    #[error("Failed to write version bytes")]
    WriteVersion(#[source] std::io::Error),
    #[error("Failed to read magic bytes")]
    ReadMagic(#[source] std::io::Error),
    #[error("Magic bytes mismatch, found {found:?}, but expected {expected:?}")]
    MagicMismatch {
        found: [u8; MAGIC_LEN],
        expected: [u8; MAGIC_LEN],
    },
    #[error("Failed to read version bytes")]
    ReadVersion(#[source] std::io::Error),
    #[error("Version mismatch, found {found}, but expected {expected}")]
    VersionMismatch { found: String, expected: String },
}

#[inline]
pub fn write_magic_and_version(
    mut magic_bytes: impl FnMut() -> [u8; MAGIC_LEN],
    w: &mut impl std::io::Write,
) -> Result<(), Error> {
    w.write_all(&magic_bytes()).map_err(Error::WriteMagic)?;
    w.write_all(&version_bytes()).map_err(Error::WriteVersion)?;
    Ok(())
}

#[inline]
pub fn read_magic_and_version(
    mut magic_bytes: impl FnMut() -> [u8; MAGIC_LEN],
    r: &mut impl std::io::Read,
) -> Result<(), Error> {
    let mut bytes: [u8; MAGIC_LEN] = [0; MAGIC_LEN];
    r.read_exact(&mut bytes).map_err(Error::ReadMagic)?;
    if bytes != magic_bytes() {
        return Err(Error::MagicMismatch {
            found: bytes,
            expected: magic_bytes(),
        });
    }

    let mut bytes: [u8; VERSION_LEN] = [0; VERSION_LEN];
    r.read_exact(&mut bytes).map_err(Error::ReadVersion)?;
    if bytes != version_bytes() {
        let found = String::from_utf8(bytes.iter().cloned().take_while(|b| *b != 0).collect())
            .unwrap_or_else(|_| format!("Failed to parse: {bytes:?}"));
        return Err(Error::VersionMismatch {
            found,
            expected: version_string(),
        });
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::{BufReader, BufWriter};

    use super::*;

    #[test]
    fn wrong_magic_number() {
        let correct_magic = [0; MAGIC_LEN];
        let wrong_magic = [1; MAGIC_LEN];

        let mut w = BufWriter::new(Vec::new());
        write_magic_and_version(|| wrong_magic, &mut w).unwrap();
        let content = w.into_inner().unwrap();
        let mut r = BufReader::new(content.as_slice());

        let Err(super::Error::MagicMismatch { found, expected }) =
            read_magic_and_version(|| correct_magic, &mut r)
        else {
            panic!();
        };
        assert_eq!(found, wrong_magic);
        assert_eq!(expected, correct_magic);
    }

    #[test]
    fn wrong_version() {
        let correct_version = version_string();

        let mut w = BufWriter::new(Vec::new());
        write_magic_and_version(|| [0; MAGIC_LEN], &mut w).unwrap();
        let mut content = w.into_inner().unwrap();

        content[MAGIC_LEN + 2] = 0;

        let mut r = BufReader::new(content.as_slice());

        let Err(super::Error::VersionMismatch { found: _, expected }) =
            read_magic_and_version(|| [0; MAGIC_LEN], &mut r)
        else {
            panic!();
        };
        assert_eq!(expected, correct_version);
    }
}
