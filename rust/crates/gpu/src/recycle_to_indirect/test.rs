// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

#[test]
fn test_simple() {
    let subgroup_size = get_subgroup_size();
    let dispatch_limit = 4;

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let indices: Vec<_> = (0..keys.len() as u32).collect();
    let counts = count_subkeys_on_cpu(3, 0, 64, subgroup_size, &indices, &keys);
    let prefixes = prefix_sum_on_cpu(&counts);

    let counts = (0..8)
        .map(|colorkey| keys.iter().filter(|key| **key == colorkey).count() as u32)
        .collect::<Vec<_>>();
    // inclusive prefix sum here
    let limits: Vec<u32> = counts
        .iter()
        .scan(0, |prefix_sum, item| {
            *prefix_sum += item;
            Some(*prefix_sum)
        })
        .collect();
    let indirect = counts
        .iter()
        .flat_map(|count| find_x_y_z_simple(dispatch_limit, *count))
        .collect::<Vec<_>>();

    assert_eq!(
        (limits, indirect),
        run_recycle_to_indirect(dispatch_limit, &prefixes),
    );
}

#[test]
fn test_random() {
    let subgroup_size = get_subgroup_size();
    let dispatch_limit = 100;

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .map(|key| key & 0b111)
        .take(10000)
        .collect();
    let indices: Vec<_> = (0..keys.len() as u32).collect();
    let counts = count_subkeys_on_cpu(3, 0, 64, subgroup_size, &indices, &keys);
    let prefixes = prefix_sum_on_cpu(&counts);

    let counts = (0..8)
        .map(|colorkey| keys.iter().filter(|key| **key == colorkey).count() as u32)
        .collect::<Vec<_>>();
    // inclusive prefix sum here
    let limits: Vec<u32> = counts
        .iter()
        .scan(0, |prefix_sum, item| {
            *prefix_sum += item;
            Some(*prefix_sum)
        })
        .collect();
    let indirect = counts
        .iter()
        .flat_map(|count| find_x_y_z_simple(dispatch_limit, *count))
        .collect::<Vec<_>>();

    assert_eq!(
        (limits, indirect),
        run_recycle_to_indirect(dispatch_limit, &prefixes),
    );
}

fn run_recycle_to_indirect(dispatch_limit: u32, prefix_sums: &[u32]) -> (Vec<u32>, Vec<u32>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let recycle_to_indirect =
        RecycleToIndirect::new(&context, RecycleToIndirectSettings { dispatch_limit });

    let buffers =
        recycle_to_indirect.create_buffers(&context, RecycleToIndirectBufferInput { prefix_sums });

    let download_limits_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_limits"),
        size: buffers.limits.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let download_indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_indirect"),
        size: buffers.indirect.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    recycle_to_indirect.compute_in_pass(
        &context,
        &mut compute_pass,
        &mut (&buffers).into(),
        &mut (),
    );

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&buffers.limits, 0, &download_limits_buffer, 0, None);
    encoder.copy_buffer_to_buffer(&buffers.indirect, 0, &download_indirect_buffer, 0, None);

    context.queue().submit([encoder.finish()]);

    let data_buffer_limits_slice = download_limits_buffer.slice(..);
    data_buffer_limits_slice.map_async(wgpu::MapMode::Read, |_| {});
    let data_buffer_indirect_slice = download_indirect_buffer.slice(..);
    data_buffer_indirect_slice.map_async(wgpu::MapMode::Read, |_| {});

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data_limits = data_buffer_limits_slice.get_mapped_range();
    let limits: &[u32] = bytemuck::cast_slice(&data_limits);
    let data_indirect = data_buffer_indirect_slice.get_mapped_range();
    let indirect: &[u32] = bytemuck::cast_slice(&data_indirect);

    (limits.to_vec(), indirect.to_vec())
}
