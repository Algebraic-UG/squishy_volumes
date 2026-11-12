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

use itertools::izip;
use nalgebra::Vector3;
use squishy_volumes_util::apply_external_force;

use super::*;

fn check(
    settings: Settings,
    input_data @ InputData {
        time_step,
        globals,
        particle_flags,
        particle_positions_and_collider_bits,
        particle_velocities,
        particle_goals,
    }: InputData,
) {
    let gpu_particle_velocites = run(settings, input_data);

    let cpu_particle_velocites: Vec<Vector3<f32>> = izip!(
        particle_flags,
        particle_positions_and_collider_bits,
        particle_velocities,
        particle_goals,
    )
    .map(
        |(flags, PositionAndColliderBits { position, .. }, velocity, goal)| {
            let to_goal = flags
                .contains(ParticleFlags::HAS_GOAL)
                .then(|| goal.xyz() - position);
            apply_external_force(&velocity.xyz(), to_goal.as_ref(), &globals, time_step)
        },
    )
    .collect();

    for (cpu, gpu) in cpu_particle_velocites
        .into_iter()
        .zip(gpu_particle_velocites)
    {
        println!("{cpu} vs {gpu}");
        check_iters_by_norm(&cpu, &gpu.xyz());
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
    let particle_goals = vec![
        Vector4::zeros(),
        Vector4::zeros(),
        Vector4::zeros(),
        Vector4::new(1., 1., 1., 0.),
    ];

    let time_step = 0.01;
    let globals = AnimatedGlobals {
        gravity_x: 0.,
        gravity_y: 0.,
        gravity_z: -9.8,
        goal_stiffness: 1000.,
        goal_damping: 0.5,
        damping: 0.,
    };

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        InputData {
            time_step,
            globals,
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_goals_positions_and_collider_bits,
            particle_velocities: &particle_velocities,
            particle_goals: &particle_goals,
        },
    );
}

#[test]
fn stiff_and_damped() {
    let particle_velocities = vec![
        Vector4::new(1., -2., 0.5, 0.),
        Vector4::new(0.3, 0.1, -0.7, 0.),
        Vector4::new(-1., 0., 2., 0.),
        Vector4::new(0., 0., 0., 0.),
    ];
    let particle_flags = vec![ParticleFlags::HAS_GOAL; 4];
    let positions = [
        Vector3::new(0.1, 0.2, 0.3),
        Vector3::new(-0.5, 0.4, 0.),
        Vector3::new(0.25, 0.25, 0.25),
        Vector3::new(0., 0., 1.),
    ];
    let particle_positions_and_collider_bits: Vec<_> = positions
        .iter()
        .map(|&position| PositionAndColliderBits {
            position,
            collider_bits: 0,
        })
        .collect();
    let particle_goals = vec![
        Vector4::new(0.1, 0.2, 0.3, 0.),
        Vector4::new(0.5, -0.4, 0.2, 0.),
        Vector4::new(0.25, 0.25, 0.250001, 0.),
        Vector4::new(0., 0., 0., 0.),
    ];

    for (goal_stiffness, goal_damping, damping) in [
        (1e7, 0., 0.),
        (1e4, 1e3, 0.),
        (1e3, 0.5, 50.),
        (-1e4, -1e3, -100.),
    ] {
        check(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            },
            InputData {
                time_step: 0.01,
                globals: AnimatedGlobals {
                    gravity_x: 0.,
                    gravity_y: 0.,
                    gravity_z: -9.8,
                    goal_stiffness,
                    goal_damping,
                    damping,
                },
                particle_flags: &particle_flags,
                particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
                particle_velocities: &particle_velocities,
                particle_goals: &particle_goals,
            },
        );
    }
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
