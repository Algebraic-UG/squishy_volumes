// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(settings: Settings, numbers: &[f32]) {
    let cpu_minimum = numbers.iter().cloned().min_by(f32::total_cmp).unwrap();

    let gpu_minimum = run(settings, numbers);

    assert_eq!(cpu_minimum, gpu_minimum);
}

fn settings() -> Settings {
    Settings {
        workgroup_size: 64.try_into().unwrap(),
        dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
    }
}

#[test]
fn single() {
    check(settings(), &[1.]);
}

#[test]
fn test_simple() {
    check(settings(), &[0., 1.]);
}

#[test]
fn test_all_positions() {
    for i in 0..1000 {
        println!("{i}");
        let mut numbers = vec![0.; 1000];
        numbers[i] = -1.;
        check(settings(), &numbers);
    }
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let numbers: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(100000)
        .collect();

    check(settings(), &numbers);
}

fn run(settings: Settings, numbers: &[f32]) -> f32 {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, numbers).unwrap();
    let find_minimum = FindMinimum::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { minimum } = find_minimum
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();
    let download = DownloadToHost::new(&context, minimum);

    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()[0]
}
