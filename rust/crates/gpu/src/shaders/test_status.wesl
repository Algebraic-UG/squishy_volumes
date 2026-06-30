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

@compute @workgroup_size(1)
fn main() {
    if atomicLoad(&status) != 0 {
        return;
    }
    atomicStore(&status, TABLE_TRIES_EXCEEDED);
}
