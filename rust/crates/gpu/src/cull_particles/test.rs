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

use super::*;

fn check(
    settings @ Settings {
        domain_min,
        domain_max,
        ..
    }: Settings,
    input_data @ InputData {
        particle_flags,
        particle_positions_and_collider_bits,
    }: InputData,
) {
    let gpu_flags = run(settings, input_data);

    let cpu_flags: Vec<ParticleFlags> =
        izip!(particle_flags, particle_positions_and_collider_bits,)
            .map(|(flags, PositionAndColliderBits { position, .. })| {
                if position.iter().zip(&domain_min).any(|(c, d)| c < d)
                    || position.iter().zip(&domain_max).any(|(c, d)| c > d)
                {
                    *flags | ParticleFlags::TOMBSTONED
                } else {
                    *flags
                }
            })
            .collect();

    for (cpu, gpu) in cpu_flags.into_iter().zip(gpu_flags) {
        assert_eq!(cpu, gpu);
    }
}

#[test]
fn simple() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    let particle_flags = vec![
        ParticleFlags::default(),
        ParticleFlags::default(),
        ParticleFlags::default(),
        ParticleFlags::HAS_GOAL,
    ];
    let particle_goals_positions_and_collider_bits = [
        Vector3::new(0.5, 0.5, 0.5),
        Vector3::new(-0.5, -0.5, -0.5),
        Vector3::new(-1.5, 0.5, 0.5),
        Vector3::new(1.5, 0.5, 0.5),
    ]
    .into_iter()
    .map(|position| PositionAndColliderBits {
        position,
        collider_bits: 0,
    })
    .collect::<Vec<_>>();

    check(
        Settings {
            workgroup_size,
            dispatch_limit,
            domain_min: Vector3::repeat(-1.),
            domain_max: Vector3::repeat(1.),
        },
        InputData {
            particle_flags: &particle_flags,
            particle_positions_and_collider_bits: &particle_goals_positions_and_collider_bits,
        },
    );
}

fn run(settings: Settings, input_data: InputData) -> Vec<ParticleFlags> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), input_data).unwrap();
    let flags = input.particle_flags.clone();
    let cull_particles = CullParticles::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = cull_particles
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [flags, context.status()]);
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [flags, status] = downloads.try_into().unwrap();

    status.to_vec::<GpuStatus>().unwrap()[0]
        .to_result(&context)
        .unwrap();

    flags.to_vec().unwrap()
}
