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

pub fn combine_matrices<X, Y, Z>(x: X, y: Y, z: Z) -> impl Iterator<Item = [[f32; 3]; 3]>
where
    X: Iterator<Item = [f32; 3]>,
    Y: Iterator<Item = [f32; 3]>,
    Z: Iterator<Item = [f32; 3]>,
{
    itertools::izip!(x, y, z).map(|(x, y, z)| [x, y, z])
}
