// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, find_dispatch_xyz};

@group(0) @binding(0)
var<storage, read> len: u32;

@group(0) @binding(1)
var<storage, read_write> new_indirect: Indirect;

@group(0) @binding(2)
var<storage, read_write> status: atomic<u32>;

override DISPATCH_LIMIT: u32;
override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(1)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
) {
    if is_error() {
        new_indirect.x = 0;
        new_indirect.y = 0;
        new_indirect.z = 0;
        new_indirect.len = 0;
        return;
    }
    set_shader_id();

    let dispatch_xzy = find_dispatch_xyz(DISPATCH_LIMIT, WORKGROUP_SIZE, len);
    new_indirect.x = dispatch_xzy.x;
    new_indirect.y = dispatch_xzy.y;
    new_indirect.z = dispatch_xzy.z;
    new_indirect.len = len;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
