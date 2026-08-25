// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
    let mut context = SHARED_CONTEXT.lock().unwrap();

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
