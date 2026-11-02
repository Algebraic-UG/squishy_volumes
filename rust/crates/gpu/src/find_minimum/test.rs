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
    let mut context = get_shared_context();

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
