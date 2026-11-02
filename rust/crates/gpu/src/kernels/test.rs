// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
    let mut context = get_shared_context();

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
