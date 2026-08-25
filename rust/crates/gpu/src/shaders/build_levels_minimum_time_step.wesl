// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, F32_MAX, TimeStepLimits};

@group(0) @binding(0)
var<storage, read> indirect: Indirect;

@group(0) @binding(1)
var<storage, read_write> intermediate: array<TimeStepLimits>;

@group(0) @binding(2)
var<storage, read_write> mininum: TimeStepLimits;

@group(0) @binding(3)
var<storage, read_write> status: atomic<u32>;

var<immediate> stride: u32;

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

    let index = global_index * stride;

    var limits: TimeStepLimits;
    limits.time_step_by_velocity = F32_MAX;
    limits.time_step_by_deformation = F32_MAX;
    limits.time_step_by_isolated = F32_MAX;
    limits.time_step_by_sound = F32_MAX;
    if index < indirect.len {
        limits = intermediate[index];
    }

    limits.time_step_by_velocity = subgroupMin(limits.time_step_by_velocity);
    limits.time_step_by_deformation = subgroupMin(limits.time_step_by_deformation);
    limits.time_step_by_isolated = subgroupMin(limits.time_step_by_isolated);
    limits.time_step_by_sound = subgroupMin(limits.time_step_by_sound);

    if index < indirect.len {
        intermediate[index] = limits;
    }

    if global_index == 0 {
        mininum = limits;
    }
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
