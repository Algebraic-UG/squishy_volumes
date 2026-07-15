// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use approx::relative_eq;

use super::*;

fn check(values: &[f32]) {
    let (linear, quadratic, cubic) = run_kernels(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
        },
        values,
    );

    for (cpu, gpu) in values.iter().cloned().map(kernel_linear).zip(linear) {
        assert!(relative_eq!(cpu, gpu));
    }
    for (cpu, gpu) in values.iter().cloned().map(kernel_quadratic).zip(quadratic) {
        assert!(relative_eq!(cpu, gpu));
    }
    for (cpu, gpu) in values.iter().cloned().map(kernel_cubic).zip(cubic) {
        assert!(relative_eq!(cpu, gpu));
    }
}

#[test]
fn test_simple() {
    let values = [-1., -0.5, 0., 0.5, 1.];
    check(&values);
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let mut values: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000)
        .collect();
    check(&values);

    values.iter_mut().for_each(|v| *v %= 4.);
    check(&values);
}

fn run_kernels(settings: Settings, values: &[f32]) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), values).unwrap();
    let kernels = Kernels::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        linear,
        quadratic,
        cubic,
    } = kernels
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [linear, quadratic, cubic]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [linear, quadratic, cubic] = downloads.try_into().unwrap();
    (
        linear.to_vec().unwrap(),
        quadratic.to_vec().unwrap(),
        cubic.to_vec().unwrap(),
    )
}
