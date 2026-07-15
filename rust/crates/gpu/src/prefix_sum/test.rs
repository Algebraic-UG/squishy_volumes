// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(settings: Settings, numbers: &[u32]) {
    let cpu_prefix_sums = prefix_sum_on_cpu(numbers);
    let cpu_total_sum: u32 = numbers.iter().sum();

    let (gpu_prefix_sums, gpu_total_sum) = run_prefix_sum(settings, numbers);

    println!("{:?}", numbers.last());
    println!("{:?} {:?}", cpu_prefix_sums.last(), gpu_prefix_sums.last());
    assert_eq!(cpu_prefix_sums, gpu_prefix_sums);
    assert_eq!(cpu_total_sum, gpu_total_sum);
}

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
        )
        .0,
    );
}

#[test]
fn test_simple_2() {
    let numbers = [2, 0, 1, 0, 4, 0, 3, 0];
    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        &numbers,
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u8>() // make sure we don't overflow
        .map(|i| i as u32)
        .take(100000)
        .collect();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        &numbers,
    );
}

fn run_prefix_sum(settings: prefix_sum::Settings, numbers: &[u32]) -> (Vec<u32>, u32) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, numbers).unwrap();
    let prefix_sum = PrefixSum::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        prefix_sums,
        total_sum,
    } = prefix_sum
        .record(
            &mut context,
            &mut (&mut encoder).into(),
            input,
            Parameters { total_sum: true },
        )
        .unwrap();
    let downloads = DownloadsToHost::new(&context, [prefix_sums, total_sum.unwrap()]);

    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [prefix_sums, total_sum] = downloads.try_into().unwrap();

    let total_sum: Vec<u32> = total_sum.to_vec().unwrap();
    assert!(total_sum.len() == 1);

    (prefix_sums.to_vec().unwrap(), total_sum[0])
}
