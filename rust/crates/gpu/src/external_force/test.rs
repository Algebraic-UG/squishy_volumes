// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(
    settings @ Settings { time_step, .. }: Settings,
    input_data @ InputData {
        gravity,
        particle_velocities,
    }: InputData,
) {
    let mut cpu_particle_velocites = particle_velocities.to_vec();
    cpu_particle_velocites
        .iter_mut()
        .for_each(|velocity| *velocity += time_step * gravity);

    let gpu_particle_velocites = run(settings, input_data);

    for (cpu, gpu) in cpu_particle_velocites
        .into_iter()
        .zip(gpu_particle_velocites)
    {
        check_iters_by_norm(&cpu.xyz(), &gpu.xyz());
    }
}

#[test]
fn simple() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let particle_velocities = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];

    let time_step = 0.01;
    let gravity = Vector4::new(0., 0., -9.8, 0.);

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
            time_step,
        },
        InputData {
            gravity,
            particle_velocities: &particle_velocities,
        },
    );
}

fn run(settings: Settings, input_data: InputData) -> Vec<Vector4<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), input_data).unwrap();
    let particle_velocities = input.particle_velocities.clone();
    let step = ExternalForce::new(&mut context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = step
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, particle_velocities);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
