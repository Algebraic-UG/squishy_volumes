// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rand::prelude::*;
use rand::rngs::ChaCha8Rng;
use std::collections::HashMap;

use nalgebra::{Matrix1x3, Matrix3, Vector3, stack};
use squishy_volumes_util::{
    first_piola_stress_inviscid, first_piola_stress_neo_hookean, lambda, mu,
};

use crate::particle_parameters::{Fluid, Host, Solid};

use super::*;

fn check(
    settings @ Settings { cell_size, .. }: Settings,
    dispatch_limit: NonZeroU32,
    input_data @ InputData {
        masses,
        initial_volumes,
        particle_parameters,
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
    }: InputData,
) {
    let grid_node_size = cell_size * 0.5;
    let scaling = 0.001 * 4. / grid_node_size.powi(2);

    let mut masses_cpu: HashMap<Vector3<i32>, Vector4<f32>> = Default::default();
    for particle_index in 0..masses.len() {
        let mass = masses[particle_index];
        let initial_volume = initial_volumes[particle_index];
        let parameters: Host = particle_parameters[particle_index].into();
        let position = positions[particle_index].xyz();
        let position_gradient: Matrix3<f32> = position_gradients[particle_index]
            .fixed_view::<3, 3>(0, 0)
            .into();
        let velocity = velocities[particle_index].xyz();
        let velocity_gradient: Matrix3<f32> = velocity_gradients[particle_index]
            .fixed_view::<3, 3>(0, 0)
            .into();

        let low_gridnode =
            (position / grid_node_size - Vector3::repeat(0.5)).map(|x| x.floor() as i32);

        let nodes = (0..3).flat_map(|i| {
            (0..3).flat_map(move |j| (0..3).map(move |k| low_gridnode + Vector3::new(i, j, k)))
        });

        for node in nodes {
            let value = masses_cpu.entry(node).or_default();
            let to_node = node.map(|c| c as f32) - position / grid_node_size;
            let weight = to_node.map(kernel_quadratic).product();
            let to_grid_node = to_node * grid_node_size;

            let mut imparted_momentum = (velocity + velocity_gradient * to_grid_node) * mass;

            let stress = match parameters {
                Host::Solid(Solid { mu, lambda, .. }) => {
                    first_piola_stress_neo_hookean(mu, lambda, &position_gradient)
                }
                Host::Fluid(Fluid {
                    exponent,
                    bulk_modulus,
                    ..
                }) => first_piola_stress_inviscid(bulk_modulus, exponent, &position_gradient),
            };
            imparted_momentum -= stress
                * (position_gradient.transpose() * (to_grid_node * (scaling * initial_volume)));

            *value += imparted_momentum.push(mass) * weight;
        }
    }

    println!("{:?}", masses_cpu);
    println!("{:?}", masses_cpu.values().collect::<Vec<_>>());

    let (addenum, blocks) = run_scatter(settings, dispatch_limit, input_data);
    let blocks_flat: Vec<Vector4<f32>> = blocks
        .iter()
        .flat_map(|block| block.nodes.iter())
        .cloned()
        .collect();

    let nodes = gpu_grid_to_cpu_grid(
        *addenum.indirect_colors_batch.last().unwrap(),
        &addenum.cell_ids,
        &addenum.cell_owns,
    );

    println!("{}", blocks_flat.len());
    println!("{blocks_flat:?}");

    for (node_id, gpu) in nodes.into_iter().zip(blocks_flat) {
        if let Some(cpu) = masses_cpu.get(&node_id.xyz()) {
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
    let settings = Settings {
        workgroup_size,
        cell_size,
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
    let settings = Settings {
        workgroup_size,
        cell_size,
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
