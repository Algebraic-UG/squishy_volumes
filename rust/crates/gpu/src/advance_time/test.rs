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

fn check(
    settings @ Settings {
        frames_per_second, ..
    }: Settings,
    time_step: f32,
    mut time: f32,
    mut step: u32,
) {
    let (gpu_status, gpu_time, gpu_step) = run(settings, time_step, time, step);

    time += time_step;
    step += 1;
    let seconds_per_frame = 1. / frames_per_second as f32;
    let reached = time > seconds_per_frame;

    let result = gpu_status.to_result(&get_shared_context());

    println!("time: {time}, reached: {reached}");
    println!("gpu_time: {gpu_time}, result: {result:?}");
    if reached {
        assert!(matches!(
            result,
            Err(GpuError::Shader(GpuShaderError::FrameTimeReached))
        ));
    } else {
        result.unwrap();
    }
    assert_eq!(time, gpu_time);
    assert_eq!(step, gpu_step);
}

#[test]
fn not_reached() {
    let workgroup_size = 64.try_into().unwrap();

    let time_step = 0.01;
    let time = 0.;
    let step = 0;
    let frames_per_second = 24;

    check(
        Settings {
            workgroup_size,
            frames_per_second,
        },
        time_step,
        time,
        step,
    );
}

#[test]
fn reached() {
    let workgroup_size = 64.try_into().unwrap();

    let time_step = 1.;
    let time = 0.;
    let step = 0;
    let frames_per_second = 24;

    check(
        Settings {
            workgroup_size,
            frames_per_second,
        },
        time_step,
        time,
        step,
    );
}

fn run(settings: Settings, time_step: f32, time: f32, step: u32) -> (GpuStatus, f32, u32) {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), time_step, time, step).unwrap();
    let advance_time = AdvanceTime::new(&mut context, settings).unwrap();

    let time = input.time.clone();
    let step = input.step.clone();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output = advance_time
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [context.status(), time, step]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [status, time, step] = downloads.try_into().unwrap();

    (
        status.to_vec().unwrap()[0],
        time.to_vec().unwrap()[0],
        step.to_vec().unwrap()[0],
    )
}
