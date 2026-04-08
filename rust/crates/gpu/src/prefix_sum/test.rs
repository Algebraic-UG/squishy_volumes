// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

#[test]
fn test_simple() {
    let numbers = [1, 1, 1, 1, 1, 1];
    assert_eq!(run_prefix_sum(64, &numbers), [0, 1, 2, 3, 4, 5]);
}

#[test]
fn test_simple_2() {
    let numbers = [2, 0, 1, 0, 4, 0, 3, 0];
    assert_eq!(prefix_sum_on_cpu(&numbers), run_prefix_sum(64, &numbers));
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u16>() // make sure we don't overflow
        .map(|i| i as u32)
        .take(1000)
        .collect();

    assert_eq!(prefix_sum_on_cpu(&numbers), run_prefix_sum(64, &numbers));
}

fn run_prefix_sum(workgroup_size: u32, numbers: &[u32]) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let prefix_sum = PrefixSum::new(&context, PrefixSumSettings { workgroup_size });
    let buffers = prefix_sum.create_buffers(&context, PrefixSumBufferInput { numbers });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.prefix_sums.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    prefix_sum.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&buffers.prefix_sums, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
