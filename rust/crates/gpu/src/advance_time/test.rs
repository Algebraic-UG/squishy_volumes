// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(
    settings @ Settings {
        frames_per_second, ..
    }: Settings,
    time_step: f32,
    mut time: f32,
) {
    let (gpu_status, gpu_time) = run(settings, time_step, time);

    time += time_step;
    let seconds_per_frame = 1. / frames_per_second as f32;
    let reached = time > seconds_per_frame;

    let result = gpu_status.to_result(&get_shared_context());

    println!("time: {time}, reached: {reached}");
    println!("gpu_time: {gpu_time}, result: {result:?}");
    if reached {
        assert!(matches!(
            result,
            Err(GpuError::Shader(GpuShaderError::FrameTimeReached { .. }))
        ));
    } else {
        result.unwrap();
    }
    assert_eq!(time, gpu_time);
}

#[test]
fn not_reached() {
    let workgroup_size = 64.try_into().unwrap();

    let time_step = 0.01;
    let time = 0.;
    let frames_per_second = 24;

    check(
        Settings {
            workgroup_size,
            frames_per_second,
        },
        time_step,
        time,
    );
}

#[test]
fn reached() {
    let workgroup_size = 64.try_into().unwrap();

    let time_step = 1.;
    let time = 0.;
    let frames_per_second = 24;

    check(
        Settings {
            workgroup_size,
            frames_per_second,
        },
        time_step,
        time,
    );
}

fn run(settings: Settings, time_step: f32, time: f32) -> (GpuStatus, f32) {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), time_step, time).unwrap();
    let advance_time = AdvanceTime::new(&mut context, settings).unwrap();

    let time = input.time.clone();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output = advance_time
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [context.status(), time]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [status, time] = downloads.try_into().unwrap();

    (status.to_vec().unwrap()[0], time.to_vec().unwrap()[0])
}
