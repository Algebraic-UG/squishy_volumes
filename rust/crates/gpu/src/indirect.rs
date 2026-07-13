// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq)]
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
