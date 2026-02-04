// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::array::from_fn;

pub const MAGIC_LEN: usize = 32;
pub const VERSION_LEN: usize = 64;
pub const HEADER_OFFSET: usize = MAGIC_LEN + VERSION_LEN;

pub fn magic_bytes() -> [u8; MAGIC_LEN] {
    const MAGIC: [char; MAGIC_LEN] = [
        'S', 'q', 'u', 'i', 's', 'h', 'y', ' ', //
        'V', 'o', 'l', 'u', 'm', 'e', 's', ' ', //
        'I', 'n', 'p', 'u', 't', ' ', //
        'F', 'i', 'l', 'e', ' ', //
        'M', 'a', 'g', 'i', 'c',
    ];
    from_fn(|i| MAGIC[i] as u8)
}

build_info::build_info!(fn build_info);

pub fn version_bytes() -> [u8; VERSION_LEN] {
    let version_string = build_info().crate_info.version.to_string();
    let bytes = version_string.as_bytes();
    assert!(bytes.len() <= VERSION_LEN, "Version string too long");
    from_fn(|i| if i < bytes.len() { bytes[i] } else { 0 })
}
