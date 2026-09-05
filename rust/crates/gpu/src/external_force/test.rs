// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use itertools::izip;
use nalgebra::Vector3;
use squishy_volumes_util::NORMALIZATION_EPS;

use super::*;

fn check(
    settings: Settings,
    input_data @ InputData {
        time,
        time_step,
        globals_start,
        globals_end,
        particle_flags,
        particle_positions_and_collider_bits,
        particle_velocities,
        particle_goals_start,
        particle_goals_end,
    }: InputData,
) {
    let gpu_particle_velocites = run(settings, input_data);

    let factor = time * settings.frames_per_second as f32;
    let AnimatedGlobals {
        gravity_x,
        gravity_y,
        gravity_z,
        goal_stiffness,
        goal_damping,
        damping,
    } = globals_start.interpolate(&globals_end, factor);

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
            *velocity -= time_step * damping * *velocity;
            *velocity += time_step * Vector4::new(gravity_x, gravity_y, gravity_z, 0.);
            if flags.contains(ParticleFlags::HAS_GOAL) {
                let goal = goal_start * (1. - factor) + goal_end * factor;
                let to_goal = goal.xyz() - position;
                let distance = to_goal.norm();
                if distance > NORMALIZATION_EPS {
                    let to_goal_dir = to_goal / distance;
                    *velocity = (goal_stiffness * distance
                        - goal_damping * to_goal_dir.dot(&velocity.xyz()))
                        * time_step
                        * to_goal_dir.push(0.);
                }
            }
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
    let globals_start = AnimatedGlobals {
        gravity_x: 0.,
        gravity_y: 0.,
        gravity_z: -9.8,
        goal_stiffness: 1000.,
        goal_damping: 0.5,
        damping: 0.,
    };
    let globals_end = globals_start;

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
            frames_per_second: 1,
        },
        InputData {
            time: 0.5,
            time_step,
            globals_start,
            globals_end,
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
