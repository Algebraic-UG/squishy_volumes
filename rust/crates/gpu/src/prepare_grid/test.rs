// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::HashSet;

use nalgebra::Vector4;

use super::*;

fn check(settings: Settings, positions: &[Vector4<f32>]) {
    let mut blocks: HashSet<Vector4<i32>> = Default::default();
    for position in positions {
        let cell = position_to_cell(settings.cell_size, position);
        for x in 0..2 {
            for y in 0..2 {
                for z in 0..2 {
                    blocks.insert(cell + Vector4::new(x, y, z, 0));
                }
            }
        }
    }

    let (indirect_cells, cell_ids, cell_owns, cell_indices) = run_prepare_grid(settings, positions);
    let num_cells = indirect_cells[0].len as usize;
    println!("num_cells: {num_cells}");
    println!("indirect_cells: {indirect_cells:?}");
    println!("cell_ids: {cell_ids:?}");
    println!("cell_indices: {cell_indices:?}");
    for &index in cell_indices.iter().take(num_cells) {
        assert!((index as usize) < num_cells);
    }

    let mut blocks_gpu: HashSet<Vector4<i32>> = Default::default();
    for block_id in cell_ids
        .into_iter()
        .zip(cell_owns)
        .take(indirect_cells[0].len as usize)
        .flat_map(|(cell, owns)| {
            println!("cell: {cell:?}, owns: {owns}");
            (0..8)
                .filter(move |block| owns & (1 << block) > 0)
                .map(move |block| cell + block_offset(block))
        })
    {
        assert!(blocks_gpu.insert(block_id));
    }

    assert_eq!(blocks, blocks_gpu);
}

#[test]
fn test_single() {
    let positions = [Vector4::zeros()];

    let cell_size = 1.;

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size,
        },
        &positions,
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

    let cell_size = 1.;

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size,
        },
        &positions,
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cell_size = 1337.;

    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000 * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size,
        },
        &positions,
    );
}

#[test]
fn test_large() {
    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 1.,
        },
        &many_positions(),
    );
}

fn run_prepare_grid(
    settings: Settings,
    positions: &[Vector4<f32>],
) -> (Vec<Indirect>, Vec<Vector4<i32>>, Vec<u32>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings.clone(), positions);
    let prepare_grid = PrepareGrid::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indirect_cells,
        cell_ids,
        cell_owns,
        cell_indices,
        ..
    } = prepare_grid
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [indirect_cells, cell_ids, cell_owns, cell_indices],
    );
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [indirect_cells, cell_ids, cell_owns, cell_indices] = downloads.try_into().unwrap();

    let mut garbage_w: Vec<Vector4<i32>> = cell_ids.to_vec();
    garbage_w.iter_mut().for_each(|v| v.w = 0);

    (
        indirect_cells.to_vec(),
        garbage_w,
        cell_owns.to_vec(),
        cell_indices.to_vec(),
    )
}
