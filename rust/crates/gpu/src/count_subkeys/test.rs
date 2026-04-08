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
    let bit_count = 2;
    let bit_offset = 0;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 1);

    assert_eq!(
        count_subkeys_on_cpu(
            bit_count,
            bit_offset,
            workgroup_size,
            subgroup_size,
            &indices,
            &keys
        ),
        run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys,),
    );
}

#[test]
fn test_simple_with_offset() {
    let bit_count = 2;
    let bit_offset = 2;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 2);

    assert_eq!(
        count_subkeys_on_cpu(
            bit_count,
            bit_offset,
            workgroup_size,
            subgroup_size,
            &indices,
            &keys
        ),
        run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys),
    );
}

#[test]
fn test_larger() {
    let bit_count = 2;
    let bit_offset = 0;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [1; 513];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 3);

    assert_eq!(
        count_subkeys_on_cpu(
            bit_count,
            bit_offset,
            workgroup_size,
            subgroup_size,
            &indices,
            &keys
        ),
        run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let bit_count = 5;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 4);

    for bit_offset in 0..5 {
        assert_eq!(
            count_subkeys_on_cpu(
                bit_count,
                bit_offset,
                workgroup_size,
                subgroup_size,
                &indices,
                &keys
            ),
            run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys),
        );
    }
}

fn run_subkey_count(
    workgroup_size: u32,
    bit_count: u32,
    bit_offset: u32,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let count_subkeys = CountSubkeys::new(
        &context,
        CountSubkeysSettings {
            workgroup_size,
            bit_count,
        },
    );

    let buffers = count_subkeys.create_buffers(&context, CountSubkeysBufferInput { indices, keys });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.counts.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    count_subkeys.compute_in_pass(
        &context,
        &mut compute_pass,
        (&buffers).into(),
        CountSubkeysParamters { bit_offset },
    );

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&buffers.counts, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
