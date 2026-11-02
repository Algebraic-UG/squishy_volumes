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

use std::num::NonZeroU32;

#[repr(C)]
#[derive(Default, Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq)]
pub struct Indirect {
    pub x: u32,
    pub y: u32,
    pub z: u32,
    pub len: u32,
}

pub struct DispatchSettings {
    pub workgroup_size: NonZeroU32,
    pub dispatch_limit: NonZeroU32,
    pub len: u32,
}

impl Indirect {
    pub fn new(
        DispatchSettings {
            workgroup_size,
            dispatch_limit,
            len,
        }: DispatchSettings,
    ) -> Self {
        let workgroup_size = workgroup_size.get();
        let dispatch_limit = dispatch_limit.get();

        let workgroup_count = len.div_ceil(workgroup_size);

        let x = workgroup_count.min(dispatch_limit);
        let y = workgroup_count.div_ceil(dispatch_limit).min(dispatch_limit);
        let z = workgroup_count
            .div_ceil(dispatch_limit * dispatch_limit)
            .min(dispatch_limit);

        Self { x, y, z, len }
    }

    pub fn workgroup_count(&self) -> u32 {
        self.x * self.y * self.z
    }

    pub fn direct(&self) -> [u32; 3] {
        [self.x, self.y, self.z]
    }
}
