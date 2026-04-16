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
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 0;
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<_> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 5);

    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        bit_count.get(),
        bit_offset,
        workgroup_size.get(),
        subgroup_size,
        &indices,
        &keys,
    );
    let prefix_sums = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count.get(), bit_offset, &indices, &keys),
        run_reorder_indices(settings, parameters, &indices, &keys, &prefix_sums),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 0;
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let mut indices: Vec<_> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 6);

    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        bit_count.get(),
        bit_offset,
        workgroup_size.get(),
        subgroup_size,
        &indices,
        &keys,
    );
    let prefix_sums = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count.get(), bit_offset, &indices, &keys),
        run_reorder_indices(settings, parameters, &indices, &keys, &prefix_sums),
    );
}

fn run_reorder_indices(
    settings: Settings,
    parameters: Parameters,
    indices: &[u32],
    keys: &[u32],
    prefix_sums: &[u32],
) -> Vec<u32> {
    let mut allocator = SHARED_ALLOCATOR.lock().unwrap();
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let input = Input::new(device, settings, indices, keys, prefix_sums);

    let reorder_indices = ReorderIndices::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    let Output { indices_out } = reorder_indices
        .compute_in_pass(
            &context,
            &mut allocator,
            &mut compute_pass,
            input,
            parameters,
        )
        .unwrap();

    let download = DownloadToHost::new(&context, indices_out);

    drop(compute_pass);

    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    download.to_vec()
}
