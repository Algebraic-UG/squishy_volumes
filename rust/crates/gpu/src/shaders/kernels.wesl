// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

//https://github.com/gfx-rs/wgpu/issues/8202
//enable subgroups;

@group(0) @binding(0)
var<storage, read_write> values: array<f32>;

@group(0) @binding(1)
var<storage, read_write> linear: array<f32>;

@group(0) @binding(2)
var<storage, read_write> quadratic: array<f32>;

@group(0) @binding(3)
var<storage, read_write> cubic: array<f32>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&values) {
        return;
    }

    let value = values[global_index];
    linear[global_index] = kernel_linear(value);
    quadratic[global_index] = kernel_quadratic(value);
    cubic[global_index] = kernel_cubic(value);
}

fn get_global_index(num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
}

fn kernel_linear(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. { return 1. - x; } else { return 0.; }
}

fn kernel_quadratic(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. / 2. {
        return 3. / 4. - x * x;
    } else if x < 3. / 2. {
        return 1. / 2. * (3. / 2. - x) * (3. / 2. - x);
    } else {
        return 0.;
    }
}

fn kernel_cubic(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. {
        return 1. / 2. * x * x * x - x * x + 2. / 3.;
    } else if x < 2. {
        return 1. / 6. * (2. - x) * (2. - x) * (2. - x);
    } else {
        return 0.;
    }
}
