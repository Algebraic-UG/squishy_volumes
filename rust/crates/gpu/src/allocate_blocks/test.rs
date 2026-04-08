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
    let owns: Vec<u32> = [
        0b00000001, 0b00000100, 0b00101000, 0b00000111, 0b00000000, 0b00000000, 0b00011000,
        0b00000000, 0b00000000, 0b00000000,
    ]
    .to_vec();

    let pops: Vec<u32> = owns.iter().cloned().map(u32::count_ones).collect();
    let workgroup_size = 64;

    assert_eq!(
        prefix_sum_on_cpu(&pops),
        run_allocate_blocks(workgroup_size, &owns),
    )
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let owns: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .take(1000)
        .map(|i| i & 0b11111111)
        .collect();

    let pops: Vec<u32> = owns.iter().cloned().map(u32::count_ones).collect();
    let workgroup_size = 64;

    assert_eq!(
        prefix_sum_on_cpu(&pops),
        run_allocate_blocks(workgroup_size, &owns),
    )
}

fn run_allocate_blocks(workgroup_size: u32, owns: &[u32]) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let allocate_blocks = AllocateBlocks::new(
        &context,
        AllocateBlocksSettings {
            workgroup_size,
            prefix_sum: PrefixSumSettings { workgroup_size },
        },
    );
    let buffers = allocate_blocks.create_buffers(&context, AllocateBlocksBufferInput { owns });

    let downloads = DownloadsToHost::new(&context, [(&buffers.prefix_sum.prefix_sums, "offsets")]);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    allocate_blocks.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [offsets] = downloads.try_into().unwrap();
    offsets.to_vec()
}
