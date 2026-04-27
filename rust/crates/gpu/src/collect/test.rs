// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
    dispatch_limit: NonZeroU32,
    input_data: scatter::InputData,
) {
    let indices = sort_positions_into_cells_on_cpu(
        &(0..input_data.positions.len() as u32).collect::<Vec<_>>(),
        input_data.positions,
        cell_size,
    );

    fn permute<T: Clone>(indices: &[u32], to_permute: &[T]) -> Vec<T> {
        indices
            .iter()
            .map(|&index| to_permute[index as usize].clone())
            .collect()
    }

    let masses = permute(&indices, input_data.masses);
    let initial_volumes = permute(&indices, input_data.initial_volumes);
    let particle_parameters = permute(&indices, input_data.particle_parameters);
    let positions = permute(&indices, input_data.positions);
    let position_gradients = permute(&indices, input_data.position_gradients);
    let velocities = permute(&indices, input_data.velocities);
    let velocity_gradients = permute(&indices, input_data.velocity_gradients);

    let input_data = scatter::InputData {
        masses: &masses,
        initial_volumes: &initial_volumes,
        particle_parameters: &particle_parameters,
        positions: &positions,
        position_gradients: &position_gradients,
        velocities: &velocities,
        velocity_gradients: &velocity_gradients,
    };

    let grid_cpu = scatter_on_cpu(cell_size, time_step, input_data.clone());
    let (positions_cpu, position_gradients_cpu, velocities_cpu, velocity_gradients_cpu) =
        collect_on_cpu(cell_size, time_step, input_data.clone(), grid_cpu);

    let (positions_gpu, position_gradients_gpu, velocities_gpu, velocity_gradients_gpu) =
        run_collect(settings, dispatch_limit, input_data);

    println!("positions:");
    for (cpu, gpu) in positions_cpu.into_iter().zip(positions_gpu) {
        println!("{cpu:?} vs {gpu:?}");
        check_iters(cpu.iter(), gpu.iter());
    }
    println!("position gradients:");
    for (cpu, gpu) in position_gradients_cpu
        .into_iter()
        .zip(position_gradients_gpu)
    {
        println!("{cpu:?} vs {gpu:?}");
        check_iters(cpu.iter(), gpu.fixed_view::<3, 3>(0, 0).iter());
    }
    println!("velocities:");
    for (cpu, gpu) in velocities_cpu.into_iter().zip(velocities_gpu) {
        check_iters(cpu.iter(), gpu.iter());
    }
    println!("velocity gradients:");
    for (cpu, gpu) in velocity_gradients_cpu
        .into_iter()
        .zip(velocity_gradients_gpu)
    {
        check_iters(cpu.iter(), gpu.fixed_view::<3, 3>(0, 0).iter());
    }
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let cell_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        cell_size,
        time_step,
    };

    check(
        settings,
        dispatch_limit,
        scatter::InputData {
            masses: &[1.],
            initial_volumes: &[1.],
            particle_parameters: &[Host::Solid(Solid {
                mu: mu(1000., 0.3),
                lambda: lambda(1000., 0.3),
                viscosity: None,
                sand_alpha: None,
            })
            .into()],
            positions: &[Vector4::zeros()],
            position_gradients: &[stack![
                Matrix3::identity() * 2.;
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
    let cell_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        cell_size,
        time_step,
    };

    let positions = many_positions();
    let n = positions.len();

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
        dispatch_limit,
        scatter::InputData {
            masses: &masses,
            initial_volumes: &initial_volumes,
            particle_parameters: &particle_parameters,
            positions: &positions,
            position_gradients: &position_gradients,
            velocities: &velocities,
            velocity_gradients: &velocity_gradients,
        },
    );
}

fn run_collect(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    data: scatter::InputData,
) -> (
    Vec<Vector4<f32>>,
    Vec<Matrix4x3<f32>>,
    Vec<Vector4<f32>>,
    Vec<Matrix4x3<f32>>,
) {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let subgroup_size = context.subgroup_size();

    let input = Input::new(
        context.device(),
        settings,
        dispatch_limit,
        subgroup_size,
        data,
    );
    let scatter = Collect::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
    } = scatter
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            positions,
            position_gradients,
            velocities,
            velocity_gradients,
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
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
    ] = downloads.try_into().unwrap();
    (
        positions.to_vec(),
        position_gradients.to_vec(),
        velocities.to_vec(),
        velocity_gradients.to_vec(),
    )
}
