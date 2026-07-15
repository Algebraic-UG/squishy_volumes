// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix3, Vector3, stack};
use squishy_volumes_file_frame::SpecificParticleParameters;
use squishy_volumes_util::{lambda, mu};

use super::*;

fn check(
    settings @ Settings {
        time_step,
        grid_node_size,
        ..
    }: Settings,
    input_data: InputData,
) {
    let InputData {
        gravity: _, //TODO
        particle_parameters,
        particle_goals_start: _, // TODO
        particle_goals_end: _,   // TODO
        variable_particle_input:
            VariableParticleInputData {
                particle_flags,
                particle_positions_and_collider_bits,
                particle_position_gradients,
                particle_velocities,
                particle_velocity_gradients,
            },
        collider_input: _, // TODO
    } = &input_data;

    let (node_ids_and_collider_bits, contributor_offsets, contributors) = contributors_on_cpu(
        settings.grid_node_size,
        particle_positions_and_collider_bits,
    );
    let particle_tmp = prepare_tmp_on_cpu(
        settings.grid_node_size,
        settings.time_step,
        prepare_tmp::InputData {
            particle_flags,
            particle_parameters,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        },
    );

    let node_momentums = scatter_on_cpu(
        grid_node_size,
        scatter::InputData {
            contributor_offsets: &contributor_offsets,
            contributors: &contributors,
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            particle_tmp: &particle_tmp,
        },
    );

    collect_on_cpu(
        grid_node_size,
        time_step,
        collect::InputData {
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            node_momentums: &node_momentums,
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        },
    );

    let OutputData {
        particle_positions_and_collider_bits: _, // TODO
        particle_position_gradients: _,          // TODO
        particle_velocities: _,                  // TODO
        particle_velocity_gradients: _,          // TODO
        indirect_nodes: _,                       // TODO
        node_ids_and_collider_bits: _,           // TODO
        node_momentums: _,                       // TODO
    } = run(settings, input_data.clone());

    todo!()
    /*
    println!("positions_and_collider_bits:");
    for (cpu, gpu) in positions_cpu.into_iter().zip(positions_gpu) {
        check_iters(cpu.xyz().iter(), gpu.xyz().iter());
    }
    println!("position gradients:");
    for (cpu, gpu) in position_gradients_cpu
        .into_iter()
        .zip(position_gradients_gpu)
    {
        check_iters(
            cpu.fixed_view::<3, 3>(0, 0).iter(),
            gpu.fixed_view::<3, 3>(0, 0).iter(),
        );
    }
    println!("velocities:");
    for (cpu, gpu) in velocities_cpu.into_iter().zip(velocities_gpu) {
        check_iters(cpu.xyz().iter(), gpu.xyz().iter());
    }
    println!("velocity gradients:");
    for (cpu, gpu) in velocity_gradients_cpu
        .into_iter()
        .zip(velocity_gradients_gpu)
    {
        check_iters(
            cpu.fixed_view::<3, 3>(0, 0).iter(),
            gpu.fixed_view::<3, 3>(0, 0).iter(),
        );
    }

    println!("indirect nodes:");
    println!("node_ids_and_collider_bits:");
    println!("node_momentums:");
        */
}

#[test]
fn specific() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 0.5;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        time_step,
        grid_node_size,
        forget_distance: grid_node_size * 2.2,
        accept_distance: grid_node_size * 2.,
        table_tries: 50,
    };

    let particle_positions_and_collider_bits = specific_positions_and_collider_bits();
    let n = particle_positions_and_collider_bits.len();

    let particle_parameters = ParticleParameters {
        mass: 1.,
        initial_volume: 1.,
        viscosity: None,
        specific: SpecificParticleParameters::Solid {
            mu: mu(1000., 0.3).unwrap(),
            lambda: lambda(1000., 0.3).unwrap(),
            sand_alpha: None,
        },
    };

    check(
        settings,
        InputData {
            gravity: Vector4::new(0., 0., -9.8, 0.),
            particle_parameters: &vec![particle_parameters; n],
            particle_goals_start: &vec![Vector4::zeros(); n],
            particle_goals_end: &vec![Vector4::zeros(); n],
            variable_particle_input: VariableParticleInputData {
                particle_flags: &vec![ParticleFlags::IS_SOLID; n],
                particle_positions_and_collider_bits: &particle_positions_and_collider_bits,
                particle_position_gradients: &vec![
                    stack![
                        Matrix3::identity();
                        Matrix1x3::zeros()
                    ];
                    n
                ],
                particle_velocities: &vec![Vector4::zeros(); n],
                particle_velocity_gradients: &vec![
                    stack![
                        Matrix3::zeros();
                        Matrix1x3::zeros()
                    ];
                    n
                ],
            },
            collider_input: None,
        },
    )
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 0.5;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        time_step,
        grid_node_size,
        forget_distance: grid_node_size * 2.2,
        accept_distance: grid_node_size * 2.,
        table_tries: 50,
    };

    let particle_parameters = ParticleParameters {
        mass: 1.,
        initial_volume: 1.,
        viscosity: None,
        specific: SpecificParticleParameters::Solid {
            mu: mu(1000., 0.3).unwrap(),
            lambda: lambda(1000., 0.3).unwrap(),
            sand_alpha: None,
        },
    };

    check(
        settings,
        InputData {
            gravity: Vector4::new(0., 0., -9.8, 0.),
            particle_parameters: &[particle_parameters],
            particle_goals_start: &[Vector4::zeros()],
            particle_goals_end: &[Vector4::zeros()],
            variable_particle_input: VariableParticleInputData {
                particle_flags: &[ParticleFlags::IS_SOLID],
                particle_positions_and_collider_bits: &[PositionAndColliderBits {
                    position: Vector3::zeros(),
                    collider_bits: 0,
                }],
                particle_position_gradients: &[stack![
                    Matrix3::identity();
                    Matrix1x3::zeros()
                ]],
                particle_velocities: &[Vector4::zeros()],
                particle_velocity_gradients: &[stack![
                    Matrix3::zeros();
                    Matrix1x3::zeros()
                ]],
            },
            collider_input: None,
        },
    );
}

/*
#[test]
fn test_many_random_props() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let cell_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
        cell_size,
        time_step,
    };

    let positions = many_positions();
    let n = positions.len();

    let indices: Vec<_> = (0..n as u32).collect();
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let masses = (0..n)
        .map(|_| rng.random_range(0.01..0.05))
        .collect::<Vec<_>>();
    let initial_volumes = (0..n)
        .map(|_| rng.random_range(0.01..0.05))
        .collect::<Vec<_>>();

    let particle_parameters = test_lame_parameters()
        .chain(test_lame_parameters())
        .cycle()
        .take(n)
        .map(Into::into)
        .collect::<Vec<_>>();
    let position_gradients = test_position_gradients_random(n)
        .into_iter()
        .map(|m| stack![m; Matrix1x3::zeros()])
        .collect::<Vec<_>>();
    let velocities = (0..n)
        .map(|_| {
            Vector4::new(
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                0.,
            )
        })
        .collect::<Vec<_>>();
    let velocity_gradients = (0..n)
        .map(|_| {
            stack![
                Matrix3::new(
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                    rng.random_range(-1.0..1.),
                );
                Matrix1x3::zeros()
            ]
        })
        .collect::<Vec<_>>();

    check(
        settings,
        InputData {
            indices: &indices,
            masses: &masses,
            initial_volumes: &initial_volumes,
            parameters: &particle_parameters,
            positions: &positions,
            position_gradients: &position_gradients,
            velocities: &velocities,
            velocity_gradients: &velocity_gradients,
        },
    );
}
*/

fn run(settings: Settings, data: InputData) -> OutputData {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let max_num_grid_nodes = (data.particle_parameters.len() as u32 * 27)
        .try_into()
        .unwrap();
    let input = Input::new(
        context.device(),
        settings.accept_distance,
        16,
        settings.clone(),
        data,
    )
    .unwrap();

    let variable_particle_input = input.variable_particle_input.clone();

    let step = Step::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indirect_nodes,
        node_ids_and_collider_bits,
        node_momentums,
    } = step
        .record(
            &mut context,
            &mut (&mut encoder).into(),
            input,
            Parameters {
                factor: 0.5,
                max_num_grid_nodes,
            },
        )
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            variable_particle_input.particle_positions_and_collider_bits,
            variable_particle_input.particle_position_gradients,
            variable_particle_input.particle_velocities,
            variable_particle_input.particle_velocity_gradients,
            indirect_nodes,
            node_ids_and_collider_bits,
            node_momentums,
            context.status(),
        ],
    );
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
        indirect_nodes,
        node_ids_and_collider_bits,
        node_momentums,
        status,
    ] = downloads.try_into().unwrap();

    status.to_vec::<GpuStatus>().unwrap()[0]
        .to_result(&context)
        .unwrap();

    OutputData {
        particle_positions_and_collider_bits: particle_positions_and_collider_bits
            .to_vec()
            .unwrap(),
        particle_position_gradients: particle_position_gradients.to_vec().unwrap(),
        particle_velocities: particle_velocities.to_vec().unwrap(),
        particle_velocity_gradients: particle_velocity_gradients.to_vec().unwrap(),
        indirect_nodes: indirect_nodes.to_vec().unwrap(),
        node_ids_and_collider_bits: node_ids_and_collider_bits.to_vec().unwrap(),
        node_momentums: node_momentums.to_vec().unwrap(),
    }
}
