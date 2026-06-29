// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index};
import super::svd::{Svd, svd};

@group(0) @binding(0)
var<storage, read_write> matrices: array<mat3x3f>;

@group(0) @binding(1)
var<storage, read_write> svds: array<Svd>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&matrices) {
        return;
    }

    svds[global_index] = svd(matrices[global_index]);
}
