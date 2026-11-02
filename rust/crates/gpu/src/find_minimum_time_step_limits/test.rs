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

fn check(settings: Settings, limits: &[TimeStepLimits]) {
    let cpu_minimum = limits
        .iter()
        .fold(TimeStepLimits::default(), |minimum, item| minimum.min(item));
    let gpu_minimum = run(settings, limits);

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
    check(
        settings(),
        &[TimeStepLimits {
            time_step_by_velocity: 1.0,
            time_step_by_deformation: 20000000.0,
            time_step_by_isolated: 0.02309401,
            time_step_by_sound: 0.027255382,
        }],
    );
}

#[test]
fn test_simple() {
    check(
        settings(),
        &[
            TimeStepLimits {
                time_step_by_velocity: 0.,
                time_step_by_deformation: 1.,
                time_step_by_isolated: 0.,
                time_step_by_sound: 1.,
            },
            TimeStepLimits {
                time_step_by_velocity: 1.,
                time_step_by_deformation: 0.,
                time_step_by_isolated: 1.,
                time_step_by_sound: 0.,
            },
        ],
    );
}

#[test]
fn test_all_positions() {
    for i in 0..1000 {
        println!("{i}");
        let mut limits = vec![TimeStepLimits::default(); 1000];
        limits[i].time_step_by_deformation = -1.;
        check(settings(), &limits);
    }
}

fn run(settings: Settings, limits: &[TimeStepLimits]) -> TimeStepLimits {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), settings, limits).unwrap();
    let find_minimum = FindMinimumTimeStepLimits::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        minimum_time_step_limits,
    } = find_minimum
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();
    let download = DownloadToHost::new(&context, minimum_time_step_limits);

    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()[0]
}
