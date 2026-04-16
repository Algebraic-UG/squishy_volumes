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
    assert_eq!(
        vec![0, 1, 2, 3, 4, 5],
        run_prefix_sum(
            prefix_sum::Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: 10.try_into().unwrap(),
            },
            &numbers
        ),
    );
}

#[test]
fn test_simple_2() {
    let numbers = [2, 0, 1, 0, 4, 0, 3, 0];
    assert_eq!(
        prefix_sum_on_cpu(&numbers),
        run_prefix_sum(
            prefix_sum::Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            },
            &numbers
        )
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u16>() // make sure we don't overflow
        .map(|i| i as u32)
        .take(10000)
        .collect();

    assert_eq!(
        prefix_sum_on_cpu(&numbers),
        run_prefix_sum(
            prefix_sum::Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            },
            &numbers
        )
    );
}

fn run_prefix_sum(settings: prefix_sum::Settings, numbers: &[u32]) -> Vec<u32> {
    let mut allocator = SHARED_ALLOCATOR.lock().unwrap();
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let input = Input::new(device, settings, numbers);
    let prefix_sum = PrefixSum::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    let Output { prefix_sums } = prefix_sum
        .compute_in_pass(
            &context,
            &mut allocator,
            &mut compute_pass,
            input,
            Parameters,
        )
        .unwrap();
    let download = DownloadToHost::new(&context, prefix_sums);

    drop(compute_pass);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    download.to_vec()
}
