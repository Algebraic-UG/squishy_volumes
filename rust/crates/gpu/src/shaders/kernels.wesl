// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, kernel_linear, kernel_quadratic, kernel_cubic};

@group(0) @binding(0)
var<storage, read_write> values: array<f32>;

@group(0) @binding(1)
var<storage, read_write> linear: array<f32>;

@group(0) @binding(2)
var<storage, read_write> quadratic: array<f32>;

@group(0) @binding(3)
var<storage, read_write> cubic: array<f32>;

@group(0) @binding(4)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
    if global_index >= arrayLength(&values) {
        return;
    }

    let value = values[global_index];
    linear[global_index] = kernel_linear(value);
    quadratic[global_index] = kernel_quadratic(value);
    cubic[global_index] = kernel_cubic(value);
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
