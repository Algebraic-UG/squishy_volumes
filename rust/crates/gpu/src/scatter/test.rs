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

#[test]
fn test_simple() {
    let positions = [Vector4::<f32>::zeros()];
    let cell_size = 1.;
    let grid_node_size = cell_size * 0.5;

    let mut masses: HashMap<Vector3<i32>, f32> = Default::default();
    for position in positions {
        let low_gridnode =
            (position.xyz() / grid_node_size - Vector3::repeat(0.5)).map(|x| x.floor() as i32);

        let nodes = (0..3).flat_map(|i| {
            (0..3).flat_map(move |j| (0..3).map(move |k| low_gridnode + Vector3::new(i, j, k)))
        });

        for node in nodes {
            let mass = masses.entry(node).or_default();
            let to_node = node.map(|c| c as f32) - position.xyz() / grid_node_size;
            let weight = to_node.map(kernel_quadratic).product();
            *mass += weight;
        }
    }

    let masses: Vec<_> = masses.values().collect();
    println!("{masses:?}");

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let blocks = run_scatter(
        Settings {
            workgroup_size,
            cell_size,
        },
        dispatch_limit,
        &positions,
    );
    println!("{blocks:?}");
    panic!()
}

fn run_scatter(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    positions: &[Vector4<f32>],
) -> Vec<Block> {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let subgroup_size = context.subgroup_size();

    let input = Input::new(
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

    download.to_vec()
}
