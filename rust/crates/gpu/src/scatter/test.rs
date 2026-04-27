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
    input_data: InputData,
) {
    let grid_cpu = scatter_on_cpu(cell_size, time_step, input_data.clone());

    println!("{:?}", grid_cpu);
    println!("{:?}", grid_cpu.values().collect::<Vec<_>>());

    let (addendum, blocks) = run_scatter(settings, dispatch_limit, input_data);
    let blocks_flat: Vec<Vector4<f32>> = blocks
        .iter()
        .flat_map(|block| block.nodes.iter())
        .cloned()
        .collect();

    let nodes = gpu_grid_to_cpu_grid(
        addendum.indirect_cells,
        &addendum.cell_ids,
        &addendum.cell_owns,
    );

    println!("{}", blocks_flat.len());
    println!("{blocks_flat:?}");

    for (node_id, gpu) in nodes.into_iter().zip(blocks_flat) {
        if let Some(cpu) = grid_cpu.get(&node_id.xyz()) {
            println!("both have {:?}", node_id.xyz());
            println!("{} vs {}", cpu, gpu);
            check_iters(cpu.iter(), gpu.iter());
        } else {
            assert_eq!(gpu, Vector4::zeros());
        }
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
        InputData {
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
        InputData {
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

fn run_scatter(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    data: InputData,
) -> (InputAddendum, Vec<Block>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let subgroup_size = context.subgroup_size();

    let (input, addendum) = Input::new(
        context.device(),
        settings,
        dispatch_limit,
        subgroup_size,
        data,
    );
    println!("{addendum:?}");
    let scatter = Scatter::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { blocks } = scatter
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, blocks);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    (addendum, download.to_vec())
}
