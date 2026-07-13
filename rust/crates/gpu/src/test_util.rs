// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZeroU32, sync::Mutex};

use crate::{DispatchSettings, GpuContext, Indirect, PositionAndColliderBits};

use approx::assert_relative_eq;
// Maybe we can avoid this once this is fixed?
// https://github.com/gfx-rs/wgpu/issues/5270
// https://github.com/KhronosGroup/Vulkan-Loader/issues/1863
use lazy_static::lazy_static;
use nalgebra::{Matrix3, Vector3, Vector4};
lazy_static! {
    pub static ref SHARED_CONTEXT: Mutex<GpuContext> = Mutex::new({
        let mut context = GpuContext::new().unwrap();
        context
            .setup_allocator(None, 10000000, "test allocator", true)
            .unwrap();
        context
            .setup_indirect_allocator(2048, "test indirect allocator", true)
            .unwrap();
        context
    });
}

// This one is ugly.
// We're emulating the behaviour on the GPU which is influenced by the fact that we have to
// dispatch in multiples of the workgroup size.
// Given that the workgroup size is a multiple of the subgroup size, there can be subgroups
// that are entirely out of bounds.
pub fn count_subkeys_on_cpu(
    dispatch_limit: u32,
    bit_count: u32,
    bit_offset: u32,
    workgroup_size: u32,
    subgroup_size: u32,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let counter_count = 1 << bit_count;
    let mask = counter_count - 1;

    // this part calculates how many counters there will be
    let subgroups_per_workgroup = workgroup_size / subgroup_size;
    let actual_workgroup_count = Indirect::new(DispatchSettings {
        workgroup_size: workgroup_size.try_into().unwrap(),
        dispatch_limit: dispatch_limit.try_into().unwrap(),
        len: keys.len() as u32,
    })
    .workgroup_count();
    let num_subgroups = subgroups_per_workgroup * actual_workgroup_count;
    let num_counter = actual_workgroup_count * subgroups_per_workgroup * 2u32.pow(bit_count);

    // we can chunk through the subgroups but it's not enough
    let mut counts: Vec<u32> = indices
        .chunks(subgroup_size as usize)
        .flat_map(|chunk| {
            (0..counter_count).map(|counter| {
                chunk
                    .iter()
                    .map(|index| keys[*index as usize])
                    .map(|number| (number >> bit_offset) & mask)
                    .filter(move |sub_key| *sub_key == counter)
                    .count() as u32
            })
        })
        .collect();

    // the missing entries correspond to subgroups that are entirely out of bounds
    // they count all zero
    counts.resize(num_counter as usize, 0);

    // this part transposes the data
    let counts = &counts;
    (0..counter_count)
        .flat_map(|counter| {
            (0..num_subgroups)
                .map(move |subgroup| counts[(subgroup * counter_count + counter) as usize])
        })
        .collect()
}

pub fn get_subgroup_size() -> NonZeroU32 {
    GpuContext::new().unwrap().subgroup_size()
}

pub fn sort_on_cpu_by_bits(
    bit_count: u32,
    bit_offset: u32,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let counter_count = 1 << bit_count;
    let mask = counter_count - 1;

    let mut indices = indices.to_vec();

    indices.sort_by_key(|index| {
        let key = keys[*index as usize];
        (key >> bit_offset) & mask
    });

    indices
}

pub fn many_positions() -> Vec<Vector4<f32>> {
    [
        Vector3::new(-0.875000, -0.875000, 0.875000),
        Vector3::new(-0.875000, -0.625000, 0.875000),
        Vector3::new(-0.875000, -0.375000, 0.875000),
        Vector3::new(-0.875000, -0.125000, 0.875000),
        Vector3::new(-0.875000, 0.125000, 0.875000),
        Vector3::new(-0.875000, 0.375000, 0.875000),
        Vector3::new(-0.875000, 0.625000, 0.875000),
        Vector3::new(-0.875000, 0.875000, 0.875000),
        Vector3::new(-0.875000, -0.875000, 0.625000),
        Vector3::new(-0.875000, -0.625000, 0.625000),
        Vector3::new(-0.875000, -0.375000, 0.625000),
        Vector3::new(-0.875000, -0.125000, 0.625000),
        Vector3::new(-0.875000, 0.125000, 0.625000),
        Vector3::new(-0.875000, 0.375000, 0.625000),
        Vector3::new(-0.875000, 0.625000, 0.625000),
        Vector3::new(-0.875000, 0.875000, 0.625000),
        Vector3::new(-0.875000, -0.875000, 0.375000),
        Vector3::new(-0.875000, -0.625000, 0.375000),
        Vector3::new(-0.875000, -0.375000, 0.375000),
        Vector3::new(-0.875000, -0.125000, 0.375000),
        Vector3::new(-0.875000, 0.125000, 0.375000),
        Vector3::new(-0.875000, 0.375000, 0.375000),
        Vector3::new(-0.875000, 0.625000, 0.375000),
        Vector3::new(-0.875000, 0.875000, 0.375000),
        Vector3::new(-0.875000, -0.875000, 0.125000),
        Vector3::new(-0.875000, -0.625000, 0.125000),
        Vector3::new(-0.875000, -0.375000, 0.125000),
        Vector3::new(-0.875000, -0.125000, 0.125000),
        Vector3::new(-0.875000, 0.125000, 0.125000),
        Vector3::new(-0.875000, 0.375000, 0.125000),
        Vector3::new(-0.875000, 0.625000, 0.125000),
        Vector3::new(-0.875000, 0.875000, 0.125000),
        Vector3::new(-0.875000, -0.875000, -0.125000),
        Vector3::new(-0.875000, -0.625000, -0.125000),
        Vector3::new(-0.875000, -0.375000, -0.125000),
        Vector3::new(-0.875000, -0.125000, -0.125000),
        Vector3::new(-0.875000, 0.125000, -0.125000),
        Vector3::new(-0.875000, 0.375000, -0.125000),
        Vector3::new(-0.875000, 0.625000, -0.125000),
        Vector3::new(-0.875000, 0.875000, -0.125000),
        Vector3::new(-0.875000, -0.875000, -0.375000),
        Vector3::new(-0.875000, -0.625000, -0.375000),
        Vector3::new(-0.875000, -0.375000, -0.375000),
        Vector3::new(-0.875000, -0.125000, -0.375000),
        Vector3::new(-0.875000, 0.125000, -0.375000),
        Vector3::new(-0.875000, 0.375000, -0.375000),
        Vector3::new(-0.875000, 0.625000, -0.375000),
        Vector3::new(-0.875000, 0.875000, -0.375000),
        Vector3::new(-0.875000, -0.875000, -0.625000),
        Vector3::new(-0.875000, -0.625000, -0.625000),
        Vector3::new(-0.875000, -0.375000, -0.625000),
        Vector3::new(-0.875000, -0.125000, -0.625000),
        Vector3::new(-0.875000, 0.125000, -0.625000),
        Vector3::new(-0.875000, 0.375000, -0.625000),
        Vector3::new(-0.875000, 0.625000, -0.625000),
        Vector3::new(-0.875000, 0.875000, -0.625000),
        Vector3::new(-0.875000, -0.875000, -0.875000),
        Vector3::new(-0.875000, -0.625000, -0.875000),
        Vector3::new(-0.875000, -0.375000, -0.875000),
        Vector3::new(-0.875000, -0.125000, -0.875000),
        Vector3::new(-0.875000, 0.125000, -0.875000),
        Vector3::new(-0.875000, 0.375000, -0.875000),
        Vector3::new(-0.875000, 0.625000, -0.875000),
        Vector3::new(-0.875000, 0.875000, -0.875000),
        Vector3::new(-0.625000, -0.875000, 0.875000),
        Vector3::new(-0.625000, -0.625000, 0.875000),
        Vector3::new(-0.625000, -0.375000, 0.875000),
        Vector3::new(-0.625000, -0.125000, 0.875000),
        Vector3::new(-0.625000, 0.125000, 0.875000),
        Vector3::new(-0.625000, 0.375000, 0.875000),
        Vector3::new(-0.625000, 0.625000, 0.875000),
        Vector3::new(-0.625000, 0.875000, 0.875000),
        Vector3::new(-0.625000, -0.875000, 0.625000),
        Vector3::new(-0.625000, -0.625000, 0.625000),
        Vector3::new(-0.625000, -0.375000, 0.625000),
        Vector3::new(-0.625000, -0.125000, 0.625000),
        Vector3::new(-0.625000, 0.125000, 0.625000),
        Vector3::new(-0.625000, 0.375000, 0.625000),
        Vector3::new(-0.625000, 0.625000, 0.625000),
        Vector3::new(-0.625000, 0.875000, 0.625000),
        Vector3::new(-0.625000, -0.875000, 0.375000),
        Vector3::new(-0.625000, -0.625000, 0.375000),
        Vector3::new(-0.625000, -0.375000, 0.375000),
        Vector3::new(-0.625000, -0.125000, 0.375000),
        Vector3::new(-0.625000, 0.125000, 0.375000),
        Vector3::new(-0.625000, 0.375000, 0.375000),
        Vector3::new(-0.625000, 0.625000, 0.375000),
        Vector3::new(-0.625000, 0.875000, 0.375000),
        Vector3::new(-0.625000, -0.875000, 0.125000),
        Vector3::new(-0.625000, -0.625000, 0.125000),
        Vector3::new(-0.625000, -0.375000, 0.125000),
        Vector3::new(-0.625000, -0.125000, 0.125000),
        Vector3::new(-0.625000, 0.125000, 0.125000),
        Vector3::new(-0.625000, 0.375000, 0.125000),
        Vector3::new(-0.625000, 0.625000, 0.125000),
        Vector3::new(-0.625000, 0.875000, 0.125000),
        Vector3::new(-0.625000, -0.875000, -0.125000),
        Vector3::new(-0.625000, -0.625000, -0.125000),
        Vector3::new(-0.625000, -0.375000, -0.125000),
        Vector3::new(-0.625000, -0.125000, -0.125000),
        Vector3::new(-0.625000, 0.125000, -0.125000),
        Vector3::new(-0.625000, 0.375000, -0.125000),
        Vector3::new(-0.625000, 0.625000, -0.125000),
        Vector3::new(-0.625000, 0.875000, -0.125000),
        Vector3::new(-0.625000, -0.875000, -0.375000),
        Vector3::new(-0.625000, -0.625000, -0.375000),
        Vector3::new(-0.625000, -0.375000, -0.375000),
        Vector3::new(-0.625000, -0.125000, -0.375000),
        Vector3::new(-0.625000, 0.125000, -0.375000),
        Vector3::new(-0.625000, 0.375000, -0.375000),
        Vector3::new(-0.625000, 0.625000, -0.375000),
        Vector3::new(-0.625000, 0.875000, -0.375000),
        Vector3::new(-0.625000, -0.875000, -0.625000),
        Vector3::new(-0.625000, -0.625000, -0.625000),
        Vector3::new(-0.625000, -0.375000, -0.625000),
        Vector3::new(-0.625000, -0.125000, -0.625000),
        Vector3::new(-0.625000, 0.125000, -0.625000),
        Vector3::new(-0.625000, 0.375000, -0.625000),
        Vector3::new(-0.625000, 0.625000, -0.625000),
        Vector3::new(-0.625000, 0.875000, -0.625000),
        Vector3::new(-0.625000, -0.875000, -0.875000),
        Vector3::new(-0.625000, -0.625000, -0.875000),
        Vector3::new(-0.625000, -0.375000, -0.875000),
        Vector3::new(-0.625000, -0.125000, -0.875000),
        Vector3::new(-0.625000, 0.125000, -0.875000),
        Vector3::new(-0.625000, 0.375000, -0.875000),
        Vector3::new(-0.625000, 0.625000, -0.875000),
        Vector3::new(-0.625000, 0.875000, -0.875000),
        Vector3::new(-0.375000, -0.875000, 0.875000),
        Vector3::new(-0.375000, -0.625000, 0.875000),
        Vector3::new(-0.375000, -0.375000, 0.875000),
        Vector3::new(-0.375000, -0.125000, 0.875000),
        Vector3::new(-0.375000, 0.125000, 0.875000),
        Vector3::new(-0.375000, 0.375000, 0.875000),
        Vector3::new(-0.375000, 0.625000, 0.875000),
        Vector3::new(-0.375000, 0.875000, 0.875000),
        Vector3::new(-0.375000, -0.875000, 0.625000),
        Vector3::new(-0.375000, -0.625000, 0.625000),
        Vector3::new(-0.375000, -0.375000, 0.625000),
        Vector3::new(-0.375000, -0.125000, 0.625000),
        Vector3::new(-0.375000, 0.125000, 0.625000),
        Vector3::new(-0.375000, 0.375000, 0.625000),
        Vector3::new(-0.375000, 0.625000, 0.625000),
        Vector3::new(-0.375000, 0.875000, 0.625000),
        Vector3::new(-0.375000, -0.875000, 0.375000),
        Vector3::new(-0.375000, -0.625000, 0.375000),
        Vector3::new(-0.375000, -0.375000, 0.375000),
        Vector3::new(-0.375000, -0.125000, 0.375000),
        Vector3::new(-0.375000, 0.125000, 0.375000),
        Vector3::new(-0.375000, 0.375000, 0.375000),
        Vector3::new(-0.375000, 0.625000, 0.375000),
        Vector3::new(-0.375000, 0.875000, 0.375000),
        Vector3::new(-0.375000, -0.875000, 0.125000),
        Vector3::new(-0.375000, -0.625000, 0.125000),
        Vector3::new(-0.375000, -0.375000, 0.125000),
        Vector3::new(-0.375000, -0.125000, 0.125000),
        Vector3::new(-0.375000, 0.125000, 0.125000),
        Vector3::new(-0.375000, 0.375000, 0.125000),
        Vector3::new(-0.375000, 0.625000, 0.125000),
        Vector3::new(-0.375000, 0.875000, 0.125000),
        Vector3::new(-0.375000, -0.875000, -0.125000),
        Vector3::new(-0.375000, -0.625000, -0.125000),
        Vector3::new(-0.375000, -0.375000, -0.125000),
        Vector3::new(-0.375000, -0.125000, -0.125000),
        Vector3::new(-0.375000, 0.125000, -0.125000),
        Vector3::new(-0.375000, 0.375000, -0.125000),
        Vector3::new(-0.375000, 0.625000, -0.125000),
        Vector3::new(-0.375000, 0.875000, -0.125000),
        Vector3::new(-0.375000, -0.875000, -0.375000),
        Vector3::new(-0.375000, -0.625000, -0.375000),
        Vector3::new(-0.375000, -0.375000, -0.375000),
        Vector3::new(-0.375000, -0.125000, -0.375000),
        Vector3::new(-0.375000, 0.125000, -0.375000),
        Vector3::new(-0.375000, 0.375000, -0.375000),
        Vector3::new(-0.375000, 0.625000, -0.375000),
        Vector3::new(-0.375000, 0.875000, -0.375000),
        Vector3::new(-0.375000, -0.875000, -0.625000),
        Vector3::new(-0.375000, -0.625000, -0.625000),
        Vector3::new(-0.375000, -0.375000, -0.625000),
        Vector3::new(-0.375000, -0.125000, -0.625000),
        Vector3::new(-0.375000, 0.125000, -0.625000),
        Vector3::new(-0.375000, 0.375000, -0.625000),
        Vector3::new(-0.375000, 0.625000, -0.625000),
        Vector3::new(-0.375000, 0.875000, -0.625000),
        Vector3::new(-0.375000, -0.875000, -0.875000),
        Vector3::new(-0.375000, -0.625000, -0.875000),
        Vector3::new(-0.375000, -0.375000, -0.875000),
        Vector3::new(-0.375000, -0.125000, -0.875000),
        Vector3::new(-0.375000, 0.125000, -0.875000),
        Vector3::new(-0.375000, 0.375000, -0.875000),
        Vector3::new(-0.375000, 0.625000, -0.875000),
        Vector3::new(-0.375000, 0.875000, -0.875000),
        Vector3::new(-0.125000, -0.875000, 0.875000),
        Vector3::new(-0.125000, -0.625000, 0.875000),
        Vector3::new(-0.125000, -0.375000, 0.875000),
        Vector3::new(-0.125000, -0.125000, 0.875000),
        Vector3::new(-0.125000, 0.125000, 0.875000),
        Vector3::new(-0.125000, 0.375000, 0.875000),
        Vector3::new(-0.125000, 0.625000, 0.875000),
        Vector3::new(-0.125000, 0.875000, 0.875000),
        Vector3::new(-0.125000, -0.875000, 0.625000),
        Vector3::new(-0.125000, -0.625000, 0.625000),
        Vector3::new(-0.125000, -0.375000, 0.625000),
        Vector3::new(-0.125000, -0.125000, 0.625000),
        Vector3::new(-0.125000, 0.125000, 0.625000),
        Vector3::new(-0.125000, 0.375000, 0.625000),
        Vector3::new(-0.125000, 0.625000, 0.625000),
        Vector3::new(-0.125000, 0.875000, 0.625000),
        Vector3::new(-0.125000, -0.875000, 0.375000),
        Vector3::new(-0.125000, -0.625000, 0.375000),
        Vector3::new(-0.125000, -0.375000, 0.375000),
        Vector3::new(-0.125000, -0.125000, 0.375000),
        Vector3::new(-0.125000, 0.125000, 0.375000),
        Vector3::new(-0.125000, 0.375000, 0.375000),
        Vector3::new(-0.125000, 0.625000, 0.375000),
        Vector3::new(-0.125000, 0.875000, 0.375000),
        Vector3::new(-0.125000, -0.875000, 0.125000),
        Vector3::new(-0.125000, -0.625000, 0.125000),
        Vector3::new(-0.125000, -0.375000, 0.125000),
        Vector3::new(-0.125000, -0.125000, 0.125000),
        Vector3::new(-0.125000, 0.125000, 0.125000),
        Vector3::new(-0.125000, 0.375000, 0.125000),
        Vector3::new(-0.125000, 0.625000, 0.125000),
        Vector3::new(-0.125000, 0.875000, 0.125000),
        Vector3::new(-0.125000, -0.875000, -0.125000),
        Vector3::new(-0.125000, -0.625000, -0.125000),
        Vector3::new(-0.125000, -0.375000, -0.125000),
        Vector3::new(-0.125000, -0.125000, -0.125000),
        Vector3::new(-0.125000, 0.125000, -0.125000),
        Vector3::new(-0.125000, 0.375000, -0.125000),
        Vector3::new(-0.125000, 0.625000, -0.125000),
        Vector3::new(-0.125000, 0.875000, -0.125000),
        Vector3::new(-0.125000, -0.875000, -0.375000),
        Vector3::new(-0.125000, -0.625000, -0.375000),
        Vector3::new(-0.125000, -0.375000, -0.375000),
        Vector3::new(-0.125000, -0.125000, -0.375000),
        Vector3::new(-0.125000, 0.125000, -0.375000),
        Vector3::new(-0.125000, 0.375000, -0.375000),
        Vector3::new(-0.125000, 0.625000, -0.375000),
        Vector3::new(-0.125000, 0.875000, -0.375000),
        Vector3::new(-0.125000, -0.875000, -0.625000),
        Vector3::new(-0.125000, -0.625000, -0.625000),
        Vector3::new(-0.125000, -0.375000, -0.625000),
        Vector3::new(-0.125000, -0.125000, -0.625000),
        Vector3::new(-0.125000, 0.125000, -0.625000),
        Vector3::new(-0.125000, 0.375000, -0.625000),
        Vector3::new(-0.125000, 0.625000, -0.625000),
        Vector3::new(-0.125000, 0.875000, -0.625000),
        Vector3::new(-0.125000, -0.875000, -0.875000),
        Vector3::new(-0.125000, -0.625000, -0.875000),
        Vector3::new(-0.125000, -0.375000, -0.875000),
        Vector3::new(-0.125000, -0.125000, -0.875000),
        Vector3::new(-0.125000, 0.125000, -0.875000),
        Vector3::new(-0.125000, 0.375000, -0.875000),
        Vector3::new(-0.125000, 0.625000, -0.875000),
        Vector3::new(-0.125000, 0.875000, -0.875000),
        Vector3::new(0.125000, -0.875000, 0.875000),
        Vector3::new(0.125000, -0.625000, 0.875000),
        Vector3::new(0.125000, -0.375000, 0.875000),
        Vector3::new(0.125000, -0.125000, 0.875000),
        Vector3::new(0.125000, 0.125000, 0.875000),
        Vector3::new(0.125000, 0.375000, 0.875000),
        Vector3::new(0.125000, 0.625000, 0.875000),
        Vector3::new(0.125000, 0.875000, 0.875000),
        Vector3::new(0.125000, -0.875000, 0.625000),
        Vector3::new(0.125000, -0.625000, 0.625000),
        Vector3::new(0.125000, -0.375000, 0.625000),
        Vector3::new(0.125000, -0.125000, 0.625000),
        Vector3::new(0.125000, 0.125000, 0.625000),
        Vector3::new(0.125000, 0.375000, 0.625000),
        Vector3::new(0.125000, 0.625000, 0.625000),
        Vector3::new(0.125000, 0.875000, 0.625000),
        Vector3::new(0.125000, -0.875000, 0.375000),
        Vector3::new(0.125000, -0.625000, 0.375000),
        Vector3::new(0.125000, -0.375000, 0.375000),
        Vector3::new(0.125000, -0.125000, 0.375000),
        Vector3::new(0.125000, 0.125000, 0.375000),
        Vector3::new(0.125000, 0.375000, 0.375000),
        Vector3::new(0.125000, 0.625000, 0.375000),
        Vector3::new(0.125000, 0.875000, 0.375000),
        Vector3::new(0.125000, -0.875000, 0.125000),
        Vector3::new(0.125000, -0.625000, 0.125000),
        Vector3::new(0.125000, -0.375000, 0.125000),
        Vector3::new(0.125000, -0.125000, 0.125000),
        Vector3::new(0.125000, 0.125000, 0.125000),
        Vector3::new(0.125000, 0.375000, 0.125000),
        Vector3::new(0.125000, 0.625000, 0.125000),
        Vector3::new(0.125000, 0.875000, 0.125000),
        Vector3::new(0.125000, -0.875000, -0.125000),
        Vector3::new(0.125000, -0.625000, -0.125000),
        Vector3::new(0.125000, -0.375000, -0.125000),
        Vector3::new(0.125000, -0.125000, -0.125000),
        Vector3::new(0.125000, 0.125000, -0.125000),
        Vector3::new(0.125000, 0.375000, -0.125000),
        Vector3::new(0.125000, 0.625000, -0.125000),
        Vector3::new(0.125000, 0.875000, -0.125000),
        Vector3::new(0.125000, -0.875000, -0.375000),
        Vector3::new(0.125000, -0.625000, -0.375000),
        Vector3::new(0.125000, -0.375000, -0.375000),
        Vector3::new(0.125000, -0.125000, -0.375000),
        Vector3::new(0.125000, 0.125000, -0.375000),
        Vector3::new(0.125000, 0.375000, -0.375000),
        Vector3::new(0.125000, 0.625000, -0.375000),
        Vector3::new(0.125000, 0.875000, -0.375000),
        Vector3::new(0.125000, -0.875000, -0.625000),
        Vector3::new(0.125000, -0.625000, -0.625000),
        Vector3::new(0.125000, -0.375000, -0.625000),
        Vector3::new(0.125000, -0.125000, -0.625000),
        Vector3::new(0.125000, 0.125000, -0.625000),
        Vector3::new(0.125000, 0.375000, -0.625000),
        Vector3::new(0.125000, 0.625000, -0.625000),
        Vector3::new(0.125000, 0.875000, -0.625000),
        Vector3::new(0.125000, -0.875000, -0.875000),
        Vector3::new(0.125000, -0.625000, -0.875000),
        Vector3::new(0.125000, -0.375000, -0.875000),
        Vector3::new(0.125000, -0.125000, -0.875000),
        Vector3::new(0.125000, 0.125000, -0.875000),
        Vector3::new(0.125000, 0.375000, -0.875000),
        Vector3::new(0.125000, 0.625000, -0.875000),
        Vector3::new(0.125000, 0.875000, -0.875000),
        Vector3::new(0.375000, -0.875000, 0.875000),
        Vector3::new(0.375000, -0.625000, 0.875000),
        Vector3::new(0.375000, -0.375000, 0.875000),
        Vector3::new(0.375000, -0.125000, 0.875000),
        Vector3::new(0.375000, 0.125000, 0.875000),
        Vector3::new(0.375000, 0.375000, 0.875000),
        Vector3::new(0.375000, 0.625000, 0.875000),
        Vector3::new(0.375000, 0.875000, 0.875000),
        Vector3::new(0.375000, -0.875000, 0.625000),
        Vector3::new(0.375000, -0.625000, 0.625000),
        Vector3::new(0.375000, -0.375000, 0.625000),
        Vector3::new(0.375000, -0.125000, 0.625000),
        Vector3::new(0.375000, 0.125000, 0.625000),
        Vector3::new(0.375000, 0.375000, 0.625000),
        Vector3::new(0.375000, 0.625000, 0.625000),
        Vector3::new(0.375000, 0.875000, 0.625000),
        Vector3::new(0.375000, -0.875000, 0.375000),
        Vector3::new(0.375000, -0.625000, 0.375000),
        Vector3::new(0.375000, -0.375000, 0.375000),
        Vector3::new(0.375000, -0.125000, 0.375000),
        Vector3::new(0.375000, 0.125000, 0.375000),
        Vector3::new(0.375000, 0.375000, 0.375000),
        Vector3::new(0.375000, 0.625000, 0.375000),
        Vector3::new(0.375000, 0.875000, 0.375000),
        Vector3::new(0.375000, -0.875000, 0.125000),
        Vector3::new(0.375000, -0.625000, 0.125000),
        Vector3::new(0.375000, -0.375000, 0.125000),
        Vector3::new(0.375000, -0.125000, 0.125000),
        Vector3::new(0.375000, 0.125000, 0.125000),
        Vector3::new(0.375000, 0.375000, 0.125000),
        Vector3::new(0.375000, 0.625000, 0.125000),
        Vector3::new(0.375000, 0.875000, 0.125000),
        Vector3::new(0.375000, -0.875000, -0.125000),
        Vector3::new(0.375000, -0.625000, -0.125000),
        Vector3::new(0.375000, -0.375000, -0.125000),
        Vector3::new(0.375000, -0.125000, -0.125000),
        Vector3::new(0.375000, 0.125000, -0.125000),
        Vector3::new(0.375000, 0.375000, -0.125000),
        Vector3::new(0.375000, 0.625000, -0.125000),
        Vector3::new(0.375000, 0.875000, -0.125000),
        Vector3::new(0.375000, -0.875000, -0.375000),
        Vector3::new(0.375000, -0.625000, -0.375000),
        Vector3::new(0.375000, -0.375000, -0.375000),
        Vector3::new(0.375000, -0.125000, -0.375000),
        Vector3::new(0.375000, 0.125000, -0.375000),
        Vector3::new(0.375000, 0.375000, -0.375000),
        Vector3::new(0.375000, 0.625000, -0.375000),
        Vector3::new(0.375000, 0.875000, -0.375000),
        Vector3::new(0.375000, -0.875000, -0.625000),
        Vector3::new(0.375000, -0.625000, -0.625000),
        Vector3::new(0.375000, -0.375000, -0.625000),
        Vector3::new(0.375000, -0.125000, -0.625000),
        Vector3::new(0.375000, 0.125000, -0.625000),
        Vector3::new(0.375000, 0.375000, -0.625000),
        Vector3::new(0.375000, 0.625000, -0.625000),
        Vector3::new(0.375000, 0.875000, -0.625000),
        Vector3::new(0.375000, -0.875000, -0.875000),
        Vector3::new(0.375000, -0.625000, -0.875000),
        Vector3::new(0.375000, -0.375000, -0.875000),
        Vector3::new(0.375000, -0.125000, -0.875000),
        Vector3::new(0.375000, 0.125000, -0.875000),
        Vector3::new(0.375000, 0.375000, -0.875000),
        Vector3::new(0.375000, 0.625000, -0.875000),
        Vector3::new(0.375000, 0.875000, -0.875000),
        Vector3::new(0.625000, -0.875000, 0.875000),
        Vector3::new(0.625000, -0.625000, 0.875000),
        Vector3::new(0.625000, -0.375000, 0.875000),
        Vector3::new(0.625000, -0.125000, 0.875000),
        Vector3::new(0.625000, 0.125000, 0.875000),
        Vector3::new(0.625000, 0.375000, 0.875000),
        Vector3::new(0.625000, 0.625000, 0.875000),
        Vector3::new(0.625000, 0.875000, 0.875000),
        Vector3::new(0.625000, -0.875000, 0.625000),
        Vector3::new(0.625000, -0.625000, 0.625000),
        Vector3::new(0.625000, -0.375000, 0.625000),
        Vector3::new(0.625000, -0.125000, 0.625000),
        Vector3::new(0.625000, 0.125000, 0.625000),
        Vector3::new(0.625000, 0.375000, 0.625000),
        Vector3::new(0.625000, 0.625000, 0.625000),
        Vector3::new(0.625000, 0.875000, 0.625000),
        Vector3::new(0.625000, -0.875000, 0.375000),
        Vector3::new(0.625000, -0.625000, 0.375000),
        Vector3::new(0.625000, -0.375000, 0.375000),
        Vector3::new(0.625000, -0.125000, 0.375000),
        Vector3::new(0.625000, 0.125000, 0.375000),
        Vector3::new(0.625000, 0.375000, 0.375000),
        Vector3::new(0.625000, 0.625000, 0.375000),
        Vector3::new(0.625000, 0.875000, 0.375000),
        Vector3::new(0.625000, -0.875000, 0.125000),
        Vector3::new(0.625000, -0.625000, 0.125000),
        Vector3::new(0.625000, -0.375000, 0.125000),
        Vector3::new(0.625000, -0.125000, 0.125000),
        Vector3::new(0.625000, 0.125000, 0.125000),
        Vector3::new(0.625000, 0.375000, 0.125000),
        Vector3::new(0.625000, 0.625000, 0.125000),
        Vector3::new(0.625000, 0.875000, 0.125000),
        Vector3::new(0.625000, -0.875000, -0.125000),
        Vector3::new(0.625000, -0.625000, -0.125000),
        Vector3::new(0.625000, -0.375000, -0.125000),
        Vector3::new(0.625000, -0.125000, -0.125000),
        Vector3::new(0.625000, 0.125000, -0.125000),
        Vector3::new(0.625000, 0.375000, -0.125000),
        Vector3::new(0.625000, 0.625000, -0.125000),
        Vector3::new(0.625000, 0.875000, -0.125000),
        Vector3::new(0.625000, -0.875000, -0.375000),
        Vector3::new(0.625000, -0.625000, -0.375000),
        Vector3::new(0.625000, -0.375000, -0.375000),
        Vector3::new(0.625000, -0.125000, -0.375000),
        Vector3::new(0.625000, 0.125000, -0.375000),
        Vector3::new(0.625000, 0.375000, -0.375000),
        Vector3::new(0.625000, 0.625000, -0.375000),
        Vector3::new(0.625000, 0.875000, -0.375000),
        Vector3::new(0.625000, -0.875000, -0.625000),
        Vector3::new(0.625000, -0.625000, -0.625000),
        Vector3::new(0.625000, -0.375000, -0.625000),
        Vector3::new(0.625000, -0.125000, -0.625000),
        Vector3::new(0.625000, 0.125000, -0.625000),
        Vector3::new(0.625000, 0.375000, -0.625000),
        Vector3::new(0.625000, 0.625000, -0.625000),
        Vector3::new(0.625000, 0.875000, -0.625000),
        Vector3::new(0.625000, -0.875000, -0.875000),
        Vector3::new(0.625000, -0.625000, -0.875000),
        Vector3::new(0.625000, -0.375000, -0.875000),
        Vector3::new(0.625000, -0.125000, -0.875000),
        Vector3::new(0.625000, 0.125000, -0.875000),
        Vector3::new(0.625000, 0.375000, -0.875000),
        Vector3::new(0.625000, 0.625000, -0.875000),
        Vector3::new(0.625000, 0.875000, -0.875000),
        Vector3::new(0.875000, -0.875000, 0.875000),
        Vector3::new(0.875000, -0.625000, 0.875000),
        Vector3::new(0.875000, -0.375000, 0.875000),
        Vector3::new(0.875000, -0.125000, 0.875000),
        Vector3::new(0.875000, 0.125000, 0.875000),
        Vector3::new(0.875000, 0.375000, 0.875000),
        Vector3::new(0.875000, 0.625000, 0.875000),
        Vector3::new(0.875000, 0.875000, 0.875000),
        Vector3::new(0.875000, -0.875000, 0.625000),
        Vector3::new(0.875000, -0.625000, 0.625000),
        Vector3::new(0.875000, -0.375000, 0.625000),
        Vector3::new(0.875000, -0.125000, 0.625000),
        Vector3::new(0.875000, 0.125000, 0.625000),
        Vector3::new(0.875000, 0.375000, 0.625000),
        Vector3::new(0.875000, 0.625000, 0.625000),
        Vector3::new(0.875000, 0.875000, 0.625000),
        Vector3::new(0.875000, -0.875000, 0.375000),
        Vector3::new(0.875000, -0.625000, 0.375000),
        Vector3::new(0.875000, -0.375000, 0.375000),
        Vector3::new(0.875000, -0.125000, 0.375000),
        Vector3::new(0.875000, 0.125000, 0.375000),
        Vector3::new(0.875000, 0.375000, 0.375000),
        Vector3::new(0.875000, 0.625000, 0.375000),
        Vector3::new(0.875000, 0.875000, 0.375000),
        Vector3::new(0.875000, -0.875000, 0.125000),
        Vector3::new(0.875000, -0.625000, 0.125000),
        Vector3::new(0.875000, -0.375000, 0.125000),
        Vector3::new(0.875000, -0.125000, 0.125000),
        Vector3::new(0.875000, 0.125000, 0.125000),
        Vector3::new(0.875000, 0.375000, 0.125000),
        Vector3::new(0.875000, 0.625000, 0.125000),
        Vector3::new(0.875000, 0.875000, 0.125000),
        Vector3::new(0.875000, -0.875000, -0.125000),
        Vector3::new(0.875000, -0.625000, -0.125000),
        Vector3::new(0.875000, -0.375000, -0.125000),
        Vector3::new(0.875000, -0.125000, -0.125000),
        Vector3::new(0.875000, 0.125000, -0.125000),
        Vector3::new(0.875000, 0.375000, -0.125000),
        Vector3::new(0.875000, 0.625000, -0.125000),
        Vector3::new(0.875000, 0.875000, -0.125000),
        Vector3::new(0.875000, -0.875000, -0.375000),
        Vector3::new(0.875000, -0.625000, -0.375000),
        Vector3::new(0.875000, -0.375000, -0.375000),
        Vector3::new(0.875000, -0.125000, -0.375000),
        Vector3::new(0.875000, 0.125000, -0.375000),
        Vector3::new(0.875000, 0.375000, -0.375000),
        Vector3::new(0.875000, 0.625000, -0.375000),
        Vector3::new(0.875000, 0.875000, -0.375000),
        Vector3::new(0.875000, -0.875000, -0.625000),
        Vector3::new(0.875000, -0.625000, -0.625000),
        Vector3::new(0.875000, -0.375000, -0.625000),
        Vector3::new(0.875000, -0.125000, -0.625000),
        Vector3::new(0.875000, 0.125000, -0.625000),
        Vector3::new(0.875000, 0.375000, -0.625000),
        Vector3::new(0.875000, 0.625000, -0.625000),
        Vector3::new(0.875000, 0.875000, -0.625000),
        Vector3::new(0.875000, -0.875000, -0.875000),
        Vector3::new(0.875000, -0.625000, -0.875000),
        Vector3::new(0.875000, -0.375000, -0.875000),
        Vector3::new(0.875000, -0.125000, -0.875000),
        Vector3::new(0.875000, 0.125000, -0.875000),
        Vector3::new(0.875000, 0.375000, -0.875000),
        Vector3::new(0.875000, 0.625000, -0.875000),
        Vector3::new(0.875000, 0.875000, -0.875000),
    ]
    .into_iter()
    .map(|v| v.push(0.))
    .collect()
}

pub fn test_position_gradients_simple() -> Vec<Matrix3<f32>> {
    vec![
        Matrix3::identity(),
        Matrix3::from_row_slice(&[
            0., -1., 0., //
            1., 0., 0., //
            0., 0., 1., //
        ]),
        Matrix3::from_row_slice(&[
            0., 0., -1., //
            0., 1., 0., //
            1., 0., 0., //
        ]),
        Matrix3::from_row_slice(&[
            1., 0., 0., //
            0., 0., -1., //
            0., 1., 0., //
        ]),
        Matrix3::from_row_slice(&[
            3., 0., 0., //
            0., 2., 0., //
            0., 0., 1., //
        ]),
        Matrix3::from_row_slice(&[
            1., 0., 0., //
            0., 2., 0., //
            0., 0., 1., //
        ]),
        Matrix3::from_row_slice(&[
            0., -1., 0., //
            1., 0., 0., //
            0., 0., 2., //
        ]),
        Matrix3::from_row_slice(&[
            0., -1., 0., //
            2., 0., 0., //
            0., 0., 1., //
        ]),
        Matrix3::from_row_slice(&[
            0., -2., 0., //
            1., 0., 0., //
            0., 0., 1., //
        ]),
    ]
}

pub fn specific_positions_and_collider_bits() -> Vec<PositionAndColliderBits> {
    vec![
        PositionAndColliderBits {
            position: Vector3::new(-1.749918, -1.7499192, -2.8612735),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.72117, -1.7211702, -2.7797809),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6849447, -1.684945, -2.6786976),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6353235, -1.6353236, -2.5432606),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5721012, -1.5721014, -2.3767123),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5015092, -1.5015092, -2.1972873),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4255044, -1.4255046, -2.0008006),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3495772, -1.3495772, -1.7863554),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7618687, -1.3763185, -2.8402147),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7344488, -1.3535157, -2.7675931),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6994733, -1.3248272, -2.6760235),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6508806, -1.2854531, -2.5509334),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5883223, -1.2353543, -2.3942366),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5174638, -1.1790091, -2.2213063),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4401075, -1.1170511, -2.028474),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3623365, -1.0531406, -1.8157058),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7739778, -0.8730723, -2.8035579),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7472299, -0.8581247, -2.7367089),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7130349, -0.8394717, -2.6521864),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.665064, -0.81387633, -2.5355487),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6026014, -0.7813466, -2.387339),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5310258, -0.74505675, -2.2211814),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4521713, -0.7054308, -2.0334191),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3725929, -0.66434336, -1.8246614),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7851061, -0.29576927, -2.8096242),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7581058, -0.29091194, -2.7424412),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7236427, -0.28485277, -2.657562),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6752073, -0.27660704, -2.5405068),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6119592, -0.26619592, -2.3918064),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.539299, -0.25457808, -2.2252247),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4591391, -0.24172346, -2.037272),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.378301, -0.22799136, -1.8287197),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7851058, 0.29576957, -2.809624),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7581062, 0.29091236, -2.7424433),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7236428, 0.28485313, -2.6575632),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6752075, 0.27660736, -2.540508),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6119595, 0.26619616, -2.3918078),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5392992, 0.2545784, -2.2252257),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4591391, 0.24172388, -2.0372722),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3783009, 0.2279916, -1.8287197),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7739781, 0.8730725, -2.8035593),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7472302, 0.8581251, -2.73671),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7130356, 0.83947194, -2.6521888),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6650646, 0.8138766, -2.5355504),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6026015, 0.78134686, -2.3873403),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5310261, 0.7450571, -2.2211838),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4521713, 0.70543075, -2.033419),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.372593, 0.6643438, -1.8246619),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.761869, 1.3763185, -2.8402147),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7344494, 1.3535161, -2.767594),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.699474, 1.3248278, -2.6760256),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6508807, 1.2854533, -2.5509338),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5883228, 1.2353544, -2.3942366),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5174644, 1.1790093, -2.2213066),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4401076, 1.1170515, -2.0284736),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3623368, 1.053141, -1.8157065),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7499185, 1.7499183, -2.8612738),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.7211704, 1.72117, -2.7797813),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.6849447, 1.6849445, -2.6786964),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.635324, 1.6353242, -2.5432608),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5721016, 1.5721015, -2.3767116),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.5015095, 1.5015097, -2.1972888),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.4255053, 1.4255054, -2.000802),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3495773, 1.3495775, -1.7863554),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3763182, -1.7618693, -2.840215),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3535162, -1.7344491, -2.7675946),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.324827, -1.6994735, -2.6760237),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2854528, -1.6508805, -2.5509326),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2353541, -1.5883222, -2.3942366),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1790086, -1.5174639, -2.2213063),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1170512, -1.4401076, -2.0284743),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0531409, -1.3623364, -1.8157064),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3830266, -1.3830267, -2.7874506),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3615414, -1.3615416, -2.7249656),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3343921, -1.334392, -2.6456938),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2965188, -1.2965189, -2.5351872),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.247501, -1.247501, -2.3930194),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1914412, -1.1914415, -2.2313206),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.128911, -1.1289108, -2.0460894),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0638678, -1.0638679, -1.8374001),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3935131, -0.876577, -2.7290344),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3724447, -0.86262923, -2.6731691),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3457557, -0.8450964, -2.6022007),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.308327, -0.82071644, -2.5025668),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2593763, -0.78909785, -2.3719773),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2028913, -0.75304514, -2.2201948),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.139472, -0.7129881, -2.042804),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0732633, -0.67106557, -1.8401453),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.404801, -0.29704344, -2.7285998),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3832942, -0.29251924, -2.672732),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3560755, -0.28685597, -2.601824),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3179127, -0.27903086, -2.5023503),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2680188, -0.2689204, -2.3720238),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2105201, -0.25735772, -2.2205818),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1460626, -0.24432611, -2.043697),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0788754, -0.23029931, -1.8417919),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.404801, 0.2970439, -2.7286),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3832946, 0.2925198, -2.6727335),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3560753, 0.28685626, -2.601823),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3179126, 0.2790311, -2.502351),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2680188, 0.2689208, -2.372024),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2105201, 0.2573581, -2.220582),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1460626, 0.24432655, -2.0436974),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0788753, 0.23029952, -1.8417914),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3935132, 0.87657726, -2.729034),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3724452, 0.8626295, -2.6731696),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3457556, 0.84509665, -2.6022003),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3083268, 0.8207169, -2.502567),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2593763, 0.7890981, -2.371977),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2028913, 0.75304526, -2.2201955),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1394719, 0.7129885, -2.0428033),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.073263, 0.67106587, -1.8401449),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3830278, 1.3830274, -2.7874522),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3615416, 1.3615417, -2.7249641),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3343923, 1.3343921, -2.6456919),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2965194, 1.2965194, -2.5351868),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2475011, 1.2475011, -2.393018),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1914417, 1.1914419, -2.23132),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1289113, 1.1289114, -2.0460904),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0638679, 1.0638683, -1.8374017),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3763187, 1.7618688, -2.8402147),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3535159, 1.734449, -2.767593),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.3248279, 1.6994741, -2.6760256),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2854536, 1.6508808, -2.5509338),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.2353544, 1.5883223, -2.3942356),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1790092, 1.517464, -2.221306),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.1170512, 1.4401078, -2.0284743),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-1.0531409, 1.362337, -1.8157065),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8730723, -1.7739781, -2.8035583),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8581248, -1.7472297, -2.7367082),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8394716, -1.7130344, -2.6521842),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8138765, -1.6650641, -2.535549),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.78134674, -1.6026012, -2.387339),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.745057, -1.531026, -2.2211835),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.70543075, -1.452171, -2.0334177),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6643433, -1.3725924, -1.8246595),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8765772, -1.3935124, -2.7290342),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.86262953, -1.3724451, -2.67317),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.84509677, -1.3457556, -2.6022005),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.82071704, -1.3083268, -2.5025666),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.789098, -1.2593763, -2.3719783),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.75304514, -1.2028912, -2.2201955),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7129885, -1.1394719, -2.042804),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6710656, -1.0732632, -1.8401453),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.884174, -0.88417363, -2.655995),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.87048954, -0.87048906, -2.6078553),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.853208, -0.8532076, -2.5463169),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8290393, -0.82903886, -2.4589362),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7973865, -0.79738617, -2.3418396),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7609144, -0.7609142, -2.2019053),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7200745, -0.7200744, -2.0341074),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6771851, -0.6771851, -1.8388282),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8920771, -0.2997885, -2.6515422),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8780757, -0.29534042, -2.6036637),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8604113, -0.289746, -2.5424805),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.83572274, -0.28195912, -2.4556165),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8034112, -0.27179307, -2.3392177),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7662016, -0.26005048, -2.2000637),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7245765, -0.24672513, -2.0331795),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6809259, -0.23236668, -1.8389741),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.892077, 0.29978922, -2.6515424),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8780758, 0.2953411, -2.6036642),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8604115, 0.28974646, -2.5424812),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8357228, 0.28195956, -2.455617),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8034112, 0.27179357, -2.3392177),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7662012, 0.26005092, -2.2000618),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.72457653, 0.24672556, -2.0331807),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6809259, 0.23236685, -1.8389741),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.88417405, 0.88417405, -2.6559944),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8704898, 0.8704895, -2.6078553),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.85320824, 0.8532082, -2.5463183),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8290393, 0.8290391, -2.4589357),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7973866, 0.7973866, -2.3418393),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7609143, 0.76091415, -2.2019033),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7200745, 0.7200746, -2.034107),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6771852, 0.6771853, -1.8388281),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.87657744, 1.3935132, -2.729034),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.86262953, 1.3724452, -2.6731696),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.84509677, 1.3457557, -2.6022),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8207173, 1.3083271, -2.5025675),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.78909814, 1.2593765, -2.3719776),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.75304514, 1.2028913, -2.2201946),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.71298844, 1.139472, -2.0428033),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6710657, 1.0732633, -1.8401449),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8730724, 1.7739781, -2.803559),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.858125, 1.7472302, -2.7367096),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.83947194, 1.7130355, -2.6521885),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.8138766, 1.6650646, -2.5355504),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7813468, 1.6026015, -2.38734),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.745057, 1.5310262, -2.2211838),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.7054308, 1.4521717, -2.0334194),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.6643436, 1.3725932, -1.8246619),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29576984, -1.7851058, -2.8096242),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29091245, -1.7581055, -2.7424412),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2848533, -1.7236422, -2.6575608),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27660742, -1.675207, -2.5405066),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2661962, -1.6119591, -2.3918061),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.25457832, -1.5392988, -2.225225),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24172369, -1.4591389, -2.0372717),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2279913, -1.3783007, -1.8287191),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29704437, -1.4048005, -2.7286),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29252002, -1.3832939, -2.6727316),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28685656, -1.356075, -2.6018236),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27903134, -1.3179122, -2.5023494),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.26892096, -1.2680187, -2.3720245),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.25735795, -1.2105194, -2.22058),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2443265, -1.1460626, -2.0436976),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.23029928, -1.0788752, -1.8417904),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29978946, -0.89207655, -2.6515424),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2953414, -0.8780747, -2.6036634),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28974676, -0.86041087, -2.542481),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28195986, -0.8357223, -2.455616),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27179363, -0.8034109, -2.3392177),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.26005098, -0.766201, -2.2000632),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24672544, -0.7245764, -2.0331793),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.23236683, -0.6809257, -1.8389744),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.30228984, -0.30228865, -2.64639),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.297739, -0.29773754, -2.598763),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29202136, -0.2920203, -2.5379367),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28407085, -0.2840698, -2.4515986),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27370358, -0.2737029, -2.3359141),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2617456, -0.26174483, -2.1975665),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24819373, -0.24819331, -2.0316293),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.23361197, -0.23361176, -1.8385366),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.30228987, 0.30228958, -2.6463902),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29773888, 0.29773864, -2.5987623),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29202136, 0.2920211, -2.537936),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2840707, 0.28407055, -2.4515977),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27370358, 0.27370346, -2.3359137),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2617456, 0.2617453, -2.1975663),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24819374, 0.24819364, -2.031629),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.233612, 0.23361188, -1.8385366),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29978952, 0.892077, -2.6515417),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29534125, 0.87807566, -2.6036637),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28974664, 0.8604113, -2.542481),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28195977, 0.8357227, -2.4556165),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27179363, 0.8034111, -2.3392174),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.26005098, 0.7662013, -2.2000632),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24672559, 0.7245768, -2.0331812),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.23236696, 0.6809258, -1.8389734),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.29704404, 1.4048009, -2.7286005),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2925199, 1.3832943, -2.6727333),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28685644, 1.3560753, -2.6018238),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27903134, 1.3179127, -2.5023518),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.26892084, 1.2680188, -2.3720243),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.25735804, 1.21052, -2.220582),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24432646, 1.1460627, -2.043698),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.23029958, 1.0788752, -1.8417916),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2957697, 1.7851058, -2.8096247),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.2909124, 1.7581064, -2.7424443),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.28485325, 1.7236425, -2.657564),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.27660733, 1.6752071, -2.540508),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.26619622, 1.6119597, -2.39181),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.25457844, 1.5392991, -2.2252257),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.24172382, 1.4591392, -2.0372725),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(-0.22799166, 1.378301, -1.8287201),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29576868, -1.7851053, -2.8096232),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29091159, -1.7581052, -2.7424426),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28485233, -1.7236416, -2.6575613),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.27660668, -1.6752067, -2.5405066),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.26619565, -1.6119587, -2.3918064),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.25457802, -1.5392984, -2.2252252),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2417236, -1.4591389, -2.0372725),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.22799152, -1.3783005, -1.8287193),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2970428, -1.4048004, -2.7286005),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29251862, -1.3832936, -2.6727319),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28685534, -1.3560748, -2.6018236),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.27903038, -1.3179121, -2.5023508),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.26892012, -1.2680186, -2.3720248),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.25735748, -1.2105198, -2.2205825),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.24432607, -1.1460621, -2.0436969),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.23029935, -1.0788754, -1.841792),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2997877, -0.8920765, -2.6515424),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2953398, -0.87807447, -2.6036637),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2897455, -0.86041075, -2.5424814),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28195864, -0.8357222, -2.4556165),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.27179274, -0.8034107, -2.3392181),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.26005024, -0.766201, -2.2000632),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.24672495, -0.72457635, -2.0331802),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.23236664, -0.68092567, -1.8389739),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.302288, -0.3022885, -2.6463902),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29773715, -0.2977375, -2.5987623),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29201978, -0.29202014, -2.5379355),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2840694, -0.28406975, -2.4515984),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2737025, -0.27370286, -2.3359137),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.26174456, -0.26174483, -2.1975663),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2481931, -0.24819326, -2.031629),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.23361164, -0.23361179, -1.8385366),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.30228823, 0.30228966, -2.64639),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29773733, 0.29773867, -2.5987632),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29201987, 0.29202113, -2.5379353),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28406945, 0.28407055, -2.4515977),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.27370256, 0.2737034, -2.335913),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2617445, 0.2617452, -2.1975648),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2481931, 0.24819359, -2.0316288),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.23361163, 0.23361179, -1.8385354),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.299788, 0.892077, -2.651543),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29534, 0.87807554, -2.6036634),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28974566, 0.8604111, -2.54248),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28195882, 0.83572274, -2.4556162),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.27179286, 0.80341107, -2.3392172),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.26005042, 0.7662012, -2.2000625),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.24672498, 0.7245765, -2.0331802),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.23236656, 0.68092567, -1.8389733),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29704314, 1.4048005, -2.7286003),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29251906, 1.3832941, -2.6727333),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28685555, 1.3560749, -2.6018236),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.27903056, 1.3179123, -2.5023508),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2689203, 1.2680188, -2.3720248),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.25735757, 1.2105198, -2.2205818),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.24432616, 1.1460627, -2.043698),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.23029931, 1.0788753, -1.8417926),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29576892, 1.7851057, -2.8096242),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.29091185, 1.7581054, -2.7424426),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.28485265, 1.7236423, -2.6575637),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2766069, 1.675207, -2.5405076),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.26619577, 1.6119595, -2.39181),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.25457802, 1.5392987, -2.2252257),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.2417235, 1.4591389, -2.0372722),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.22799136, 1.3783008, -1.8287195),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8730717, -1.7739776, -2.80356),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.858124, -1.7472291, -2.7367105),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.83947074, -1.7130346, -2.6521883),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.81387573, -1.6650634, -2.5355506),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.78134626, -1.6026005, -2.3873394),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7450564, -1.5310254, -2.2211833),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7054306, -1.4521708, -2.0334187),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.66434354, -1.3725924, -1.824661),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8765756, -1.3935114, -2.729034),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8626279, -1.3724439, -2.6731694),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8450953, -1.3457551, -2.6022007),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.82071584, -1.308326, -2.5025673),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7890967, -1.2593757, -2.3719766),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7530446, -1.202891, -2.220195),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7129878, -1.1394715, -2.0428042),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.6710654, -1.0732628, -1.8401449),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8841724, -0.8841731, -2.6559954),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.87048805, -0.87048846, -2.6078553),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8532066, -0.8532073, -2.5463176),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.82903826, -0.82903856, -2.4589362),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.79738563, -0.7973859, -2.3418393),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7609135, -0.7609136, -2.201904),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.720074, -0.7200742, -2.0341067),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.6771849, -0.67718506, -1.8388281),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8920753, -0.29978803, -2.651541),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.87807417, -0.29534015, -2.603664),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8604099, -0.2897457, -2.5424795),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.83572173, -0.28195897, -2.4556162),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.80340993, -0.27179295, -2.339217),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7662006, -0.2600504, -2.2000625),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.72457576, -0.24672507, -2.0331783),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.6809255, -0.2323668, -1.8389735),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.89207584, 0.2997892, -2.6515415),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.87807435, 0.2953412, -2.603663),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.86041015, 0.28974643, -2.5424795),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.83572185, 0.28195965, -2.4556153),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.80341005, 0.27179357, -2.339217),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7662008, 0.26005083, -2.2000628),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7245758, 0.24672535, -2.0331783),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.6809255, 0.23236667, -1.8389738),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8841729, 0.8841737, -2.655994),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8704887, 0.8704892, -2.6078546),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8532072, 0.85320777, -2.546317),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8290384, 0.8290388, -2.4589348),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.79738575, 0.7973863, -2.341838),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.76091355, 0.76091415, -2.2019033),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7200741, 0.7200743, -2.034106),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.67718494, 0.67718506, -1.8388264),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8765762, 1.3935119, -2.7290337),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8626285, 1.3724447, -2.6731699),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8450961, 1.3457553, -2.6022),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8207163, 1.3083268, -2.502568),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7890974, 1.2593762, -2.3719776),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7530446, 1.2028912, -2.2201948),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7129878, 1.1394715, -2.0428028),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.6710654, 1.0732628, -1.8401449),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.87307197, 1.7739775, -2.803559),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8581243, 1.7472291, -2.736709),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8394715, 1.713035, -2.652189),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.8138762, 1.665064, -2.535551),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7813463, 1.6026006, -2.3873396),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7450567, 1.5310255, -2.2211833),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.7054305, 1.4521711, -2.0334182),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(0.66434324, 1.3725926, -1.8246616),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3763167, -1.7618669, -2.8402145),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3535143, -1.7344475, -2.7675939),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3248261, -1.6994729, -2.6760273),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2854521, -1.6508796, -2.550934),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2353535, -1.5883218, -2.3942368),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1790082, -1.5174633, -2.2213068),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1170508, -1.4401075, -2.028474),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0531406, -1.362336, -1.8157063),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3830256, -1.383026, -2.787451),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3615397, -1.3615402, -2.7249637),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.334391, -1.3343914, -2.645692),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2965177, -1.2965178, -2.5351863),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2475002, -1.2475003, -2.393019),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1914407, -1.1914408, -2.23132),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1289104, -1.1289103, -2.0460906),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0638674, -1.0638676, -1.8373998),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3935108, -0.8765758, -2.7290325),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3724438, -0.862628, -2.6731703),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3457541, -0.84509575, -2.6022003),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3083259, -0.82071596, -2.5025666),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2593752, -0.7890968, -2.371976),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2028905, -0.75304455, -2.2201943),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1394715, -0.7129881, -2.0428052),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0732628, -0.67106533, -1.840145),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4047995, -0.29704303, -2.7285979),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3832935, -0.29251888, -2.6727328),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.356074, -0.2868555, -2.601822),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3179114, -0.27903038, -2.5023487),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2680184, -0.26892033, -2.3720257),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2105191, -0.25735748, -2.2205808),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1460621, -0.24432623, -2.0436974),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0788751, -0.23029941, -1.8417927),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4047997, 0.297044, -2.728598),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3832937, 0.29251984, -2.6727335),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.356074, 0.28685638, -2.6018214),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3179117, 0.27903116, -2.5023484),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2680186, 0.2689208, -2.372025),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2105193, 0.25735793, -2.2205803),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1460623, 0.24432659, -2.043698),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0788751, 0.23029931, -1.8417923),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3935114, 0.8765769, -2.7290328),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3724444, 0.8626294, -2.673171),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3457547, 0.8450963, -2.6021993),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3083265, 0.8207163, -2.5025659),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2593758, 0.7890975, -2.3719761),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2028909, 0.753045, -2.2201943),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1394713, 0.712988, -2.0428028),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0732629, 0.6710656, -1.8401455),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3830265, 1.3830266, -2.7874525),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3615403, 1.3615406, -2.7249644),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3343915, 1.3343916, -2.645692),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2965184, 1.2965188, -2.5351872),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2475007, 1.2475007, -2.393019),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1914408, 1.1914408, -2.23132),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1289102, 1.1289102, -2.0460885),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0638674, 1.0638674, -1.8373994),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3763176, 1.7618678, -2.8402147),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3535153, 1.7344483, -2.7675936),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.324827, 1.6994731, -2.6760259),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.2854526, 1.6508802, -2.5509338),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.235354, 1.5883219, -2.3942368),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1790091, 1.5174639, -2.221308),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.1170508, 1.4401075, -2.028474),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.0531406, 1.3623362, -1.8157071),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7499167, -1.7499167, -2.861273),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7211692, -1.7211691, -2.7797823),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6849447, -1.6849444, -2.6787007),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6353225, -1.6353223, -2.5432596),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5721002, -1.5721002, -2.3767111),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5015091, -1.5015088, -2.1972892),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4255048, -1.4255043, -2.0008018),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3495772, -1.3495771, -1.7863554),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7618674, -1.3763176, -2.8402152),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7344472, -1.3535144, -2.767591),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6994724, -1.324826, -2.6760237),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6508787, -1.2854517, -2.5509303),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5883217, -1.2353532, -2.3942351),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5174632, -1.179008, -2.2213056),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4401073, -1.1170509, -2.0284743),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3623364, -1.0531403, -1.8157065),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7739764, -0.87307143, -2.8035567),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7472279, -0.8581238, -2.7367058),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7130346, -0.8394709, -2.6521869),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6650633, -0.8138756, -2.5355492),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6026002, -0.78134614, -2.3873386),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5310252, -0.74505645, -2.2211826),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4521711, -0.7054305, -2.0334187),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3725924, -0.66434336, -1.8246619),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7851053, -0.29576874, -2.8096232),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7581046, -0.29091153, -2.7424395),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7236414, -0.28485233, -2.6575599),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.675206, -0.27660656, -2.5405052),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6119585, -0.26619574, -2.391807),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5392983, -0.254578, -2.2252243),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4591389, -0.24172351, -2.0372722),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3783004, -0.22799148, -1.8287191),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7851053, 0.2957696, -2.8096235),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7581048, 0.2909124, -2.7424402),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7236414, 0.28485316, -2.6575596),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6752061, 0.27660728, -2.5405052),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6119586, 0.26619616, -2.391807),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5392984, 0.25457832, -2.2252243),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4591389, 0.24172361, -2.037272),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3783004, 0.22799134, -1.828719),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7739769, 0.87307227, -2.8035586),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7472285, 0.85812455, -2.7367082),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7130347, 0.83947176, -2.652188),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6650635, 0.8138763, -2.5355499),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6026005, 0.7813465, -2.3873389),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5310253, 0.74505657, -2.2211823),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4521708, 0.70543057, -2.0334175),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3725926, 0.6643434, -1.8246616),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7618678, 1.3763176, -2.8402152),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7344484, 1.3535153, -2.7675946),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6994734, 1.3248271, -2.6760266),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6508802, 1.2854526, -2.5509338),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5883218, 1.2353536, -2.3942368),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5174636, 1.179008, -2.2213063),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4401072, 1.1170505, -2.0284743),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3623363, 1.0531405, -1.8157076),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.7499176, 1.7499175, -2.861274),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.72117, 1.7211697, -2.7797825),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.6849444, 1.6849443, -2.6786988),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.635323, 1.635323, -2.5432618),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.5721009, 1.5721005, -2.3767126),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.501509, 1.501509, -2.197289),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.4255044, 1.4255044, -2.000802),
            collider_bits: 65537,
        },
        PositionAndColliderBits {
            position: Vector3::new(1.3495773, 1.3495771, -1.7863562),
            collider_bits: 65537,
        },
    ]
}

pub fn check_iters<'a>(a: impl IntoIterator<Item = &'a f32>, b: impl IntoIterator<Item = &'a f32>) {
    for (a, b) in a.into_iter().zip(b.into_iter()) {
        println!("{a} vs {b}");
        assert_relative_eq!(a, b, epsilon = 0.000001, max_relative = 0.01);
    }
}

pub fn check_iters_by_norm<'a>(
    a: impl IntoIterator<Item = &'a f32>,
    b: impl IntoIterator<Item = &'a f32>,
) {
    let mut norm_a = 0.;
    let mut norm_b = 0.;
    let mut norm_difference = 0.;
    for (a, b) in a.into_iter().zip(b.into_iter()) {
        norm_a += a * a;
        norm_b += b * b;
        norm_difference += (a - b) * (a - b);
    }

    let scale = norm_a.max(norm_b).sqrt();
    let error = norm_difference.sqrt();

    let epsilon = 1.0e-6;
    let max_relative = 0.01;

    let tolerance = epsilon + max_relative * scale;

    assert!(
        error <= tolerance,
        "error={error}, tolerance={tolerance}, relative_error={}",
        error / scale.max(epsilon),
    );
}
