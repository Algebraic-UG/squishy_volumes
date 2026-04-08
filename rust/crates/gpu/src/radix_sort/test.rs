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
    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_prefix_sort(
            RadixSortSettings {
                count_subkeys_settings: CountSubkeysSettings {
                    workgroup_size: 64,
                    bit_count: 2
                },
                prefix_sum_settings: PrefixSumSettings { workgroup_size: 64 },
                reorder_settings: ReorderSettings {
                    workgroup_size: 64,
                    bit_count: 2
                },
            },
            &indices,
            &keys,
        )
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_prefix_sort(
            RadixSortSettings {
                count_subkeys_settings: CountSubkeysSettings {
                    workgroup_size: 64,
                    bit_count: 2
                },
                prefix_sum_settings: PrefixSumSettings { workgroup_size: 64 },
                reorder_settings: ReorderSettings {
                    workgroup_size: 64,
                    bit_count: 2
                },
            },
            &indices,
            &keys,
        )
    );
}

fn run_prefix_sort(settings: RadixSortSettings, indices: &[u32], keys: &[u32]) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let radix_sort = RadixSort::new(&context, settings);

    let buffers = radix_sort.create_buffers(&context, RadixSortBufferInput { keys, indices });

    let download_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_indices"),
        size: buffers.indices_back.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let buffer_bindings = (&buffers).into();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    radix_sort.compute_in_pass_all_rounds(&context, &mut compute_pass, &buffer_bindings);

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(
        buffer_bindings.indices.front().buffer,
        0,
        &download_index_buffer,
        0,
        None,
    );

    context.queue().submit([encoder.finish()]);

    let data_buffer_index_slice = download_index_buffer.slice(..);
    data_buffer_index_slice.map_async(wgpu::MapMode::Read, |_| {});

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data_indices = data_buffer_index_slice.get_mapped_range();
    let indices: &[u32] = bytemuck::cast_slice(&data_indices);

    indices.to_vec()
}
