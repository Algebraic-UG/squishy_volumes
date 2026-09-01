// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use itertools::izip;
use nalgebra::Vector3;

use super::*;

fn check(
    settings: Settings,
    input_data @ InputData {
        time,
        time_step,
        gravity,
        particle_flags,
        particle_positions_and_collider_bits,
        particle_velocities,
        particle_goals_start,
        particle_goals_end,
    }: InputData,
) {
    let gpu_particle_velocites = run(settings, input_data);

    let factor = time * settings.frames_per_second as f32;

    let mut cpu_particle_velocites = particle_velocities.to_vec();
    izip!(
        particle_flags,
        particle_positions_and_collider_bits,
        &mut cpu_particle_velocites,
        particle_goals_start,
        particle_goals_end,
    )
    .for_each(
        |(flags, PositionAndColliderBits { position, .. }, velocity, goal_start, goal_end)| {
            if flags.contains(ParticleFlags::HAS_GOAL) {
                *velocity = ((goal_start * (1. - factor) + goal_end * factor).xyz() - position)
                    .push(0.)
                    / time_step;
                return;
            }
            *velocity += time_step * gravity;
        },
    );

    for (cpu, gpu) in cpu_particle_velocites
        .into_iter()
        .zip(gpu_particle_velocites)
    {
        println!("{cpu} vs {gpu}");
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
        Vector4::new(0., 0., 0., 0.),
    ];

    let particle_flags = vec![
        ParticleFlags::default(),
        ParticleFlags::default(),
        ParticleFlags::default(),
        ParticleFlags::HAS_GOAL,
    ];
    let particle_goals_positions_and_collider_bits = vec![
        PositionAndColliderBits {
            position: Vector3::zeros(),
            collider_bits: 0,
        };
        4
    ];
    let particle_goals_start = vec![
        Vector4::zeros(),
        Vector4::zeros(),
        Vector4::zeros(),
        Vector4::new(1., 1., 1., 0.),
    ];
    let particle_goals_end = vec![
        Vector4::zeros(),
        Vector4::zeros(),
        Vector4::zeros(),
        Vector4::new(1., 1., 2., 0.),
    ];

    let time_step = 0.01;
    let gravity = Vector4::new(0., 0., -9.8, 0.);

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
            frames_per_second: 1,
        },
        InputData {
            time: 0.5,
            time_step,
            gravity,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_goals_positions_and_collider_bits,
            particle_velocities: &particle_velocities,
            particle_goals_start: &particle_goals_start,
            particle_goals_end: &particle_goals_end,
        },
    );
}

fn run(settings: Settings, input_data: InputData) -> Vec<Vector4<f32>> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), input_data).unwrap();
    let particle_velocities = input.particle_velocities.clone();
    let external_force = ExternalForce::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = external_force
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [particle_velocities, context.status()]);
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [particle_velocities, status] = downloads.try_into().unwrap();

    status.to_vec::<GpuStatus>().unwrap()[0]
        .to_result(&context)
        .unwrap();

    particle_velocities.to_vec().unwrap()
}
