// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::io::Read;

use super::{InputError, MAGIC_LEN, VERSION_LEN, magic_bytes, version_bytes};

pub struct InputReader {
    //TODO
}

fn read_magic<R: Read>(mut r: R) -> Result<(), InputError> {
    let mut bytes: [u8; MAGIC_LEN] = [0; MAGIC_LEN];
    r.read_exact(&mut bytes)?;
    if bytes != magic_bytes() {
        Err(InputError::MagicMismatch)
    } else {
        Ok(())
    }
}

fn read_version<R: Read>(mut r: R) -> Result<(), InputError> {
    let mut bytes: [u8; VERSION_LEN] = [0; VERSION_LEN];
    r.read_exact(&mut bytes)?;
    if bytes != version_bytes() {
        let version_string =
            String::from_utf8(bytes.iter().cloned().take_while(|b| *b != 0).collect())
                .unwrap_or_else(|_| format!("Failed to parse: {bytes:?}"));
        Err(InputError::VersionMismatch(version_string))
    } else {
        Ok(())
    }
}
