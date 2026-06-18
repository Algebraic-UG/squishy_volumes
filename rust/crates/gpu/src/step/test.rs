// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

/*
use rand::prelude::*;
use rand::rngs::ChaCha8Rng;

use nalgebra::{Matrix1x3, Matrix3, stack};
use squishy_volumes_util::{lambda, mu};

use crate::particle_parameters::{Host, Solid};

use super::*;

fn check(
    settings @ Settings {
        cell_size,
        time_step,
        ..
    }: Settings,
    input_data @ InputData {
        masses,
        initial_volumes,
        parameters,
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
        ..
    }: InputData,
) {
    let permutation = sort_positions_into_cells_on_cpu(
        &(0..positions.len() as u32).collect::<Vec<_>>(),
        positions,
        cell_size,
    );
    let permutation = permutation.as_slice();

    let scatter_collect_data = scatter::InputData {
        masses,
        initial_volumes,
        particle_parameters: parameters,
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
    };
    let (positions, position_gradients, velocities, velocity_gradients) = collect_on_cpu(
        cell_size,
        time_step,
        scatter_collect_data.clone(),
        scatter_on_cpu(cell_size, time_step, scatter_collect_data.clone()),
    );

    let positions_cpu = permutation.permute(&positions);
    let position_gradients_cpu = permutation.permute(&position_gradients);
    let velocities_cpu = permutation.permute(&velocities);
    let velocity_gradients_cpu = permutation.permute(&velocity_gradients);

    let OutputData {
        positions_out: positions_gpu,
        position_gradients_out: position_gradients_gpu,
        velocities_out: velocities_gpu,
        velocity_gradients_out: velocity_gradients_gpu,
        ..
    } = run_step(settings, input_data);

    println!("positions:");
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
}

#[test]
fn test_single_undeformed() {
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

    check(
        settings,
        InputData {
            indices: &[0],
            masses: &[1.],
            initial_volumes: &[1.],
            parameters: &[Host::Solid(Solid {
                mu: mu(1000., 0.3),
                lambda: lambda(1000., 0.3),
                viscosity: None,
                sand_alpha: None,
            })
            .into()],
            positions: &[Vector4::zeros()],
            position_gradients: &[stack![
                Matrix3::identity();
                Matrix1x3::zeros()
            ]],
            velocities: &[Vector4::zeros()],
            velocity_gradients: &[stack![
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

fn run_step(settings: Settings, data: InputData) -> OutputData {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings.clone(), data);
    let step = Step::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indices_out,
        masses_out,
        initial_volumes_out,
        parameters_out,
        positions_out,
        position_gradients_out,
        velocities_out,
        velocity_gradients_out,
    } = step
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            indices_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
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
        indices_out,
        masses_out,
        initial_volumes_out,
        parameters_out,
        positions_out,
        position_gradients_out,
        velocities_out,
        velocity_gradients_out,
    ] = downloads.try_into().unwrap();
    OutputData {
        indices_out: indices_out.to_vec(),
        masses_out: masses_out.to_vec(),
        initial_volumes_out: initial_volumes_out.to_vec(),
        parameters_out: parameters_out.to_vec(),
        positions_out: positions_out.to_vec(),
        position_gradients_out: position_gradients_out.to_vec(),
        velocities_out: velocities_out.to_vec(),
        velocity_gradients_out: velocity_gradients_out.to_vec(),
    }
}
*/
