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

    assert_eq!(
        prefix_sum_on_cpu(&pops),
        run_allocate_blocks(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap()
            },
            &owns
        ),
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

    assert_eq!(
        prefix_sum_on_cpu(&pops),
        run_allocate_blocks(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap()
            },
            &owns
        ),
    )
}

fn run_allocate_blocks(settings: Settings, owns: &[u32]) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, owns);
    let allocate_blocks = AllocateBlocks::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { block_offsets, .. } = allocate_blocks
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, block_offsets);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
