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
    let workgroup_size = 64;
    let dispatch_limit = 4;

    let numbers = [0, 1, 1, 1, 1, 1, 0, 1, 1, 1];
    let prefixes = prefix_sum_on_cpu(&numbers);

    let limits = vec![numbers.into_iter().sum::<u32>()];
    let indirect = find_x_y_z_simple(dispatch_limit, limits[0].div_ceil(workgroup_size)).to_vec();

    assert_eq!(
        (limits, indirect),
        run_sum_to_indirect(workgroup_size, dispatch_limit, &prefixes),
    );
}

#[test]
fn test_random() {
    let workgroup_size = 64;
    let dispatch_limit = 100;

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<bool>()
        .map(|bit| if bit { 1 } else { 0 })
        .take(10000)
        .collect();
    let prefixes = prefix_sum_on_cpu(&numbers);

    println!("{}", numbers.last().unwrap());
    println!("{}", prefixes.last().unwrap());

    let limits = vec![numbers.into_iter().sum::<u32>()];
    let indirect = find_x_y_z_simple(dispatch_limit, limits[0].div_ceil(workgroup_size)).to_vec();

    assert_eq!(
        (limits, indirect),
        run_sum_to_indirect(workgroup_size, dispatch_limit, &prefixes),
    );
}

fn run_sum_to_indirect(
    workgroup_size: u32,
    dispatch_limit: u32,
    prefix_sums: &[u32],
) -> (Vec<u32>, Vec<u32>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let sum_to_indirect = OffsetsToIndirect::new(
        &context,
        OffsetsToIndirectSettings {
            workgroup_size,
            dispatch_limit,
        },
    );

    let buffers =
        sum_to_indirect.create_buffers(&context, OffsetsToIndirectBufferInput { prefix_sums });

    let downloads = DownloadsToHost::new(
        &context,
        [(&buffers.limits, "limits"), (&buffers.indirect, "indirect")],
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    sum_to_indirect.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);

    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [limits, indirect] = downloads.try_into().unwrap();

    (limits.to_vec(), indirect.to_vec())
}
