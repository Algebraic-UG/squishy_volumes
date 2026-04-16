// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

pub struct Allocations {
    pub indirect: Allocation,
    pub numbers: Allocation,
}

impl Allocations {
    pub fn new(
        device: &wgpu::Device,
        Settings {
            workgroup_size,
            dispatch_limit,
        }: Settings,
        numbers: &[u32],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: numbers.len() as u32,
        });

        let numbers = Allocation::new(device, "numbers", numbers);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self { indirect, numbers }
    }
}
