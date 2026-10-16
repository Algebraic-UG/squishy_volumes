// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub(crate) const APPLICATION_KEY: &str = "_ai_Application";
pub(crate) const SQUISHY_VOLUMES: &str = "Squishy Volumes";
pub(crate) const SQUISHY_VOLUMES_BLOB: &str = "Squishy Volumes Blob";

pub(crate) fn magic_bytes() -> [u8; squishy_volumes_file_util::MAGIC_LEN] {
    const MAGIC: [char; squishy_volumes_file_util::MAGIC_LEN] = [
        'S', 'q', 'u', 'i', 's', 'h', 'y', ' ', //
        'V', 'o', 'l', 'u', 'm', 'e', 's', ' ', //
        'F', 'r', 'a', 'm', 'e', ' ', //
        'F', 'i', 'l', 'e', ' ', //
        'M', 'a', 'g', 'i', 'c',
    ];
    std::array::from_fn(|i| MAGIC[i] as u8)
}

pub fn u32_to_two_f32(value: u32) -> [f32; 2] {
    [(value & 0x0000FFFF) as f32, (value >> 16) as f32]
}

pub fn two_f32_to_u32([low, high]: [f32; 2]) -> u32 {
    ((low as u32) & 0x0000FFFF) | ((high as u32) << 16)
}

pub fn split_matrices(matrices: &[[[f32; 3]; 3]]) -> [Vec<[f32; 3]>; 3] {
    let mut x = Vec::with_capacity(matrices.len());
    let mut y = Vec::with_capacity(matrices.len());
    let mut z = Vec::with_capacity(matrices.len());

    for m in matrices {
        x.push(m[0]);
        y.push(m[1]);
        z.push(m[2]);
    }

    [x, y, z]
}

pub fn combine_matrices([x, y, z]: [Vec<[f32; 3]>; 3]) -> impl Iterator<Item = [[f32; 3]; 3]> {
    itertools::izip!(x, y, z).map(|(x, y, z)| [x, y, z])
}
