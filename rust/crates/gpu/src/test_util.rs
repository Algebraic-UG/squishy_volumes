// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZeroU32, sync::Mutex};

use crate::{GpuContext, Indirect, IndirectSettings, MAX_NUM_PARTICLES};

// Maybe we can avoid this once this is fixed?
// https://github.com/gfx-rs/wgpu/issues/5270
// https://github.com/KhronosGroup/Vulkan-Loader/issues/1863
use lazy_static::lazy_static;
lazy_static! {
    pub static ref SHARED_CONTEXT: Mutex<GpuContext> = Mutex::new({
        let mut context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
        context
            .setup_allocator(1000000, "test allocator", true)
            .unwrap();
        context
            .setup_indirect_allocator(1000, "test indirect allocator", true)
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
    let actual_workgroup_count = Indirect::new(IndirectSettings {
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
    GpuContext::new(MAX_NUM_PARTICLES).unwrap().subgroup_size()
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
