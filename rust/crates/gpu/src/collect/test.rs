// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::test_data::test_position_gradients_random;

use super::*;
use nalgebra::{Matrix1x3, Matrix3, Vector3, stack};
use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

fn check(settings: Settings, input_data: InputData) {
    let gpu_output = run(settings, input_data.clone());
    let cpu_output = collect_on_cpu(settings.grid_node_size, settings.time_step, input_data);

    println!("checking positions");
    for (cpu, gpu) in cpu_output
        .particle_positions_and_collider_bits
        .into_iter()
        .zip(gpu_output.particle_positions_and_collider_bits)
    {
        check_iters(cpu.position.iter(), gpu.position.iter());
    }

    println!("checking position gradients");
    for (cpu, gpu) in cpu_output
        .particle_position_gradients
        .into_iter()
        .zip(gpu_output.particle_position_gradients)
    {
        check_iters(
            cpu.fixed_view::<3, 3>(0, 0).iter(),
            gpu.fixed_view::<3, 3>(0, 0).iter(),
        );
    }

    println!("checking velocities");
    for (cpu, gpu) in cpu_output
        .particle_velocities
        .into_iter()
        .zip(gpu_output.particle_velocities)
    {
        check_iters(cpu.xyz().iter(), gpu.xyz().iter());
    }

    println!("checking velocity gradients");
    for (cpu, gpu) in cpu_output
        .particle_velocity_gradients
        .into_iter()
        .zip(gpu_output.particle_velocity_gradients)
    {
        check_iters(
            cpu.fixed_view::<3, 3>(0, 0).iter(),
            gpu.fixed_view::<3, 3>(0, 0).iter(),
        );
    }
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
        time_step,
        table_tries: 50,
    };

    let node_ids = [
        Vector3::new(0, 0, 0),
        Vector3::new(0, 0, 1),
        Vector3::new(0, 0, 2),
        Vector3::new(0, 1, 0),
        Vector3::new(0, 1, 1),
        Vector3::new(0, 1, 2),
        Vector3::new(0, 2, 0),
        Vector3::new(0, 2, 1),
        Vector3::new(0, 2, 2),
        Vector3::new(1, 0, 0),
        Vector3::new(1, 0, 1),
        Vector3::new(1, 0, 2),
        Vector3::new(1, 1, 0),
        Vector3::new(1, 1, 1),
        Vector3::new(1, 1, 2),
        Vector3::new(1, 2, 0),
        Vector3::new(1, 2, 1),
        Vector3::new(1, 2, 2),
        Vector3::new(2, 0, 0),
        Vector3::new(2, 0, 1),
        Vector3::new(2, 0, 2),
        Vector3::new(2, 1, 0),
        Vector3::new(2, 1, 1),
        Vector3::new(2, 1, 2),
        Vector3::new(2, 2, 0),
        Vector3::new(2, 2, 1),
        Vector3::new(2, 2, 2),
    ];
    let node_ids_and_collider_bits = node_ids
        .into_iter()
        .map(|node_id| NodeIdAndColliderBits {
            node_id,
            collider_bits: 0,
        })
        .collect::<Vec<_>>();
    let node_momentums = vec![Vector4::zeros(); 27];

    check(
        settings,
        InputData {
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            node_momentums: &node_momentums,
            particle_positions_and_collider_bits: &[PositionAndColliderBits {
                position: Vector3::repeat(0.6),
                collider_bits: 0,
            }],
            #[allow(clippy::toplevel_ref_arg)]
            particle_position_gradients: &[stack![
                Matrix3::identity();
                Matrix1x3::zeros()
            ]],
            particle_velocities: &[Vector4::zeros()],
            #[allow(clippy::toplevel_ref_arg)]
            particle_velocity_gradients: &[stack![
                Matrix3::zeros();
                Matrix1x3::zeros()
            ]],
        },
    );
}

#[test]
fn test_many_random_props() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 0.5;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
        time_step,
        table_tries: 50,
    };

    let positions = many_positions();
    let n = positions.len();
    let positions_and_collider_bits = positions
        .into_iter()
        .map(|position| PositionAndColliderBits {
            position: position.xyz(),
            collider_bits: 0,
        })
        .collect::<Vec<_>>();

    let mut rng = ChaCha8Rng::seed_from_u64(42);

    #[allow(clippy::toplevel_ref_arg)]
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
    #[allow(clippy::toplevel_ref_arg)]
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

    let node_ids_and_collider_bits = get_node_set(grid_node_size, &positions_and_collider_bits)
        .into_iter()
        .collect::<Vec<_>>();
    let node_momentums = (0..node_ids_and_collider_bits.len())
        .map(|_| {
            Vector4::new(
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                rng.random_range(-1.0..1.),
                rng.random_range(0.1..10.),
            )
        })
        .collect::<Vec<_>>();

    check(
        settings,
        collect::InputData {
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            node_momentums: &node_momentums,
            particle_positions_and_collider_bits: &positions_and_collider_bits,
            particle_position_gradients: &position_gradients,
            particle_velocities: &velocities,
            particle_velocity_gradients: &velocity_gradients,
        },
    );
}

fn run(settings: Settings, input_data: InputData) -> OutputData {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), input_data).unwrap();
    let particle_positions_and_collider_bits = input.particle_positions_and_collider_bits.clone();
    let particle_position_gradients = input.particle_position_gradients.clone();
    let particle_velocities = input.particle_velocities.clone();
    let particle_velocity_gradients = input.particle_velocity_gradients.clone();

    let collect = Collect::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = collect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
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
    ] = downloads.try_into().unwrap();

    OutputData {
        particle_positions_and_collider_bits: particle_positions_and_collider_bits
            .to_vec()
            .unwrap(),
        particle_position_gradients: particle_position_gradients.to_vec().unwrap(),
        particle_velocities: particle_velocities.to_vec().unwrap(),
        particle_velocity_gradients: particle_velocity_gradients.to_vec().unwrap(),
    }
}
