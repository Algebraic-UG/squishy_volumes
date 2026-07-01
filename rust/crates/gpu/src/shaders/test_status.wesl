// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::TABLE_TRIES_EXCEEDED;

@group(0) @binding(0)
var<storage, read_write> status: atomic<u32>;

override SHADER_ID: u32;

@compute @workgroup_size(1)
fn main() {
    if is_error() {
        return;
    }
    set_shader_id();
    atomicOr(&status, TABLE_TRIES_EXCEEDED);
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
