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
    let mut context = SHARED_CONTEXT.lock().unwrap();

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
