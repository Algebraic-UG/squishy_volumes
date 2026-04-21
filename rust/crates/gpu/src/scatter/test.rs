// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashMap;

use nalgebra::Vector3;

use super::*;

fn check(
    settings @ Settings { cell_size, .. }: Settings,
    dispatch_limit: NonZeroU32,
    positions: &[Vector4<f32>],
) {
    let grid_node_size = cell_size * 0.5;

    let mut masses_cpu: HashMap<Vector3<i32>, f32> = Default::default();
    for position in positions {
        let low_gridnode =
            (position.xyz() / grid_node_size - Vector3::repeat(0.5)).map(|x| x.floor() as i32);

        let nodes = (0..3).flat_map(|i| {
            (0..3).flat_map(move |j| (0..3).map(move |k| low_gridnode + Vector3::new(i, j, k)))
        });

        for node in nodes {
            let mass = masses_cpu.entry(node).or_default();
            let to_node = node.map(|c| c as f32) - position.xyz() / grid_node_size;
            let weight = to_node.map(kernel_quadratic).product();
            *mass += weight;
        }
    }

    println!("{:?}", masses_cpu);
    println!("{:?}", masses_cpu.values().collect::<Vec<_>>());

    let (addenum, blocks) = run_scatter(settings, dispatch_limit, positions);
    let masses: Vec<f32> = blocks
        .iter()
        .flat_map(|block| block.nodes.iter().map(|node| node.w))
        .collect();

    let nodes = gpu_grid_to_cpu_grid(
        *addenum.indirect_colors_batch.last().unwrap(),
        &addenum.cell_ids,
        &addenum.cell_owns,
    );

    //assert_eq!(masses.len(), nodes.len());
    println!("{}", masses.len());
    println!("{masses:?}");

    for (node_id, mass) in nodes.into_iter().zip(masses) {
        if let Some(cpu) = masses_cpu.get(&node_id.xyz()) {
            println!("both have {:?}", node_id.xyz());
            assert_eq!(*cpu, mass);
        } else {
            assert!(mass == 0.);
        }
    }
}

#[test]
fn test_single() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let cell_size = 1.;
    let settings = Settings {
        workgroup_size,
        cell_size,
    };

    check(settings, dispatch_limit, &[Vector4::zeros()]);
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(cell_size, 0., 0., 0.)],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(0., cell_size, 0., 0.)],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(0., 0., cell_size, 0.)],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(cell_size, cell_size, cell_size, 0.)],
    );
    panic!()
}

#[test]
fn test_two() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let cell_size = 1.;
    let settings = Settings {
        workgroup_size,
        cell_size,
    };
    check(settings, dispatch_limit, &[Vector4::zeros(); 2]);
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(cell_size, 0., 0., 0.); 2],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(0., cell_size, 0., 0.); 2],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(0., 0., cell_size, 0.); 2],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(cell_size, cell_size, cell_size, 0.); 2],
    );
}

#[test]
fn test_two_colors() {
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
        &[
            Vector4::zeros(),
            Vector4::new(cell_size, cell_size, cell_size, 0.),
        ],
    );
}

#[test]
fn test_100() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let cell_size = 1.;
    let settings = Settings {
        workgroup_size,
        cell_size,
    };
    check(settings, dispatch_limit, &vec![Vector4::zeros(); 100]);
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(cell_size, 0., 0., 0.); 100],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(0., cell_size, 0., 0.); 100],
    );
    check(
        settings,
        dispatch_limit,
        &[Vector4::new(0., 0., cell_size, 0.); 100],
    );
}

#[test]
fn test_simple() {
    let positions = [
        Vector4::new(-0.5, -0.5, -0.5, 0.),
        Vector4::new(-0.5, -0.5, 0.5, 0.),
        Vector4::new(-0.5, 0.5, -0.5, 0.),
        Vector4::new(-0.5, 0.5, 0.5, 0.),
        Vector4::new(0.5, -0.5, -0.5, 0.),
        Vector4::new(0.5, -0.5, 0.5, 0.),
        Vector4::new(0.5, 0.5, -0.5, 0.),
        Vector4::new(0.5, 0.5, 0.5, 0.),
    ];
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let cell_size = 1.;
    check(
        Settings {
            workgroup_size,
            cell_size,
        },
        dispatch_limit,
        &positions,
    );
}

fn run_scatter(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    positions: &[Vector4<f32>],
) -> (InputAddendum, Vec<Block>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let subgroup_size = context.subgroup_size();

    let (input, addendum) = Input::new(
        context.device(),
        settings,
        dispatch_limit,
        subgroup_size,
        positions,
    );
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
