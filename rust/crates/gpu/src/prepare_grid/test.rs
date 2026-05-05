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
fn specific() {
    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 1.,
        },
        &[
            Vector4::new(-0.875, -0.875, -0.375, 5.6557635e32),
            Vector4::new(-0.875, -0.625, -0.375, -1.3383657e18),
            Vector4::new(-0.875, -0.375, -0.375, 0.015089896),
            Vector4::new(-0.875, -0.875, -0.625, -6.899305e-17),
            Vector4::new(-0.875, -0.625, -0.625, -1.2928989e-5),
            Vector4::new(-0.875, -0.375, -0.625, -0.036406733),
            Vector4::new(-0.875, -0.875, -0.875, -7.4354676e-33),
            Vector4::new(-0.875, -0.625, -0.875, 0.07318702),
            Vector4::new(-0.875, -0.375, -0.875, -1.0253879e-26),
            Vector4::new(-0.625, -0.875, -0.375, 1531479300.0),
            Vector4::new(-0.625, -0.625, -0.375, -4.4637773e-19),
            Vector4::new(-0.625, -0.375, -0.375, -2.3404627e-14),
            Vector4::new(-0.625, -0.875, -0.625, 6.1836534),
            Vector4::new(-0.625, -0.625, -0.625, -7.1751076e26),
            Vector4::new(-0.625, -0.375, -0.625, 1.596117e-26),
            Vector4::new(-0.625, -0.875, -0.875, -980615.5),
            Vector4::new(-0.625, -0.625, -0.875, -803490.9),
            Vector4::new(-0.625, -0.375, -0.875, 3.3206186e37),
            Vector4::new(-0.375, -0.875, -0.375, -3.8380856e22),
            Vector4::new(-0.375, -0.625, -0.375, -0.0002111866),
            Vector4::new(-0.375, -0.375, -0.375, 8.258342e-10),
            Vector4::new(-0.375, -0.875, -0.625, 56344506000.0),
            Vector4::new(-0.375, -0.625, -0.625, -1.994094e16),
            Vector4::new(-0.375, -0.375, -0.625, 7.154341),
            Vector4::new(-0.375, -0.875, -0.875, 5.758584e27),
            Vector4::new(-0.375, -0.625, -0.875, -1.6068397e-38),
            Vector4::new(-0.375, -0.375, -0.875, -1.158806e-25),
            Vector4::new(-0.875, -0.875, 0.625, 9.4520766e-14),
            Vector4::new(-0.875, -0.625, 0.625, -0.01482379),
            Vector4::new(-0.875, -0.375, 0.625, -1.6269866e27),
            Vector4::new(-0.875, -0.875, 0.375, -2.4268404e38),
            Vector4::new(-0.875, -0.625, 0.375, 6.798238e-24),
            Vector4::new(-0.875, -0.375, 0.375, 5.5379978e-18),
            Vector4::new(-0.875, -0.875, 0.125, 8.620416e21),
            Vector4::new(-0.875, -0.625, 0.125, -1.4980901e-21),
            Vector4::new(-0.875, -0.375, 0.125, 2.5482842e25),
            Vector4::new(-0.875, -0.875, -0.125, -1.6540205e22),
            Vector4::new(-0.875, -0.625, -0.125, -186517880000.0),
            Vector4::new(-0.875, -0.375, -0.125, 1.4981209e-25),
            Vector4::new(-0.625, -0.875, 0.625, 1.4854409e-15),
            Vector4::new(-0.625, -0.625, 0.625, 1.3071643e-21),
            Vector4::new(-0.625, -0.375, 0.625, 1.0539069e35),
            Vector4::new(-0.625, -0.875, 0.375, 3.1216874e-30),
            Vector4::new(-0.625, -0.625, 0.375, -174945540000000.0),
            Vector4::new(-0.625, -0.375, 0.375, -5.3001603e36),
            Vector4::new(-0.625, -0.875, 0.125, 8.699024e23),
            Vector4::new(-0.625, -0.625, 0.125, 85841140.0),
            Vector4::new(-0.625, -0.375, 0.125, 0.00023858502),
            Vector4::new(-0.625, -0.875, -0.125, 14450603.0),
            Vector4::new(-0.625, -0.625, -0.125, 1.1441868e-17),
            Vector4::new(-0.625, -0.375, -0.125, -1.1554132e20),
            Vector4::new(-0.375, -0.875, 0.625, -3.5859024e-17),
            Vector4::new(-0.375, -0.625, 0.625, -1049958700000000.0),
            Vector4::new(-0.375, -0.375, 0.625, 0.00047597213),
            Vector4::new(-0.375, -0.875, 0.375, -3.192066e28),
            Vector4::new(-0.375, -0.625, 0.375, 0.0417496),
            Vector4::new(-0.375, -0.375, 0.375, -3.1832382e-16),
            Vector4::new(-0.375, -0.875, 0.125, -3.6466553e-31),
            Vector4::new(-0.375, -0.625, 0.125, -1482626.3),
            Vector4::new(-0.375, -0.375, 0.125, -0.40324593),
            Vector4::new(-0.375, -0.875, -0.125, -9.2533e-27),
            Vector4::new(-0.375, -0.625, -0.125, 1.02287075e-13),
            Vector4::new(-0.375, -0.375, -0.125, -157919350000000.0),
            Vector4::new(-0.875, -0.875, 0.875, -1.5272429e29),
            Vector4::new(-0.875, -0.625, 0.875, -1553431300000.0),
            Vector4::new(-0.875, -0.375, 0.875, 5.4035867e-27),
            Vector4::new(-0.625, -0.875, 0.875, -0.10758484),
            Vector4::new(-0.625, -0.625, 0.875, -6.4793446e-31),
            Vector4::new(-0.625, -0.375, 0.875, 1.1478064e16),
            Vector4::new(-0.375, -0.875, 0.875, -6.3786694e19),
            Vector4::new(-0.375, -0.625, 0.875, 37298876000.0),
            Vector4::new(-0.375, -0.375, 0.875, 3.864502e-15),
            Vector4::new(-0.875, -0.125, -0.375, -2.9302714e22),
            Vector4::new(-0.875, 0.125, -0.375, -839619.7),
            Vector4::new(-0.875, 0.375, -0.375, 1.0607183e-30),
            Vector4::new(-0.875, 0.625, -0.375, 2.0828854e-14),
            Vector4::new(-0.875, -0.125, -0.625, -4.5253557e34),
            Vector4::new(-0.875, 0.125, -0.625, -2.1298498e30),
            Vector4::new(-0.875, 0.375, -0.625, -23898935000.0),
            Vector4::new(-0.875, 0.625, -0.625, -1.847841),
            Vector4::new(-0.875, -0.125, -0.875, 0.004805898),
            Vector4::new(-0.875, 0.125, -0.875, 6.72473e-24),
            Vector4::new(-0.875, 0.375, -0.875, 8.0330265e-16),
            Vector4::new(-0.875, 0.625, -0.875, 10391.016),
            Vector4::new(-0.625, -0.125, -0.375, -1.8729222e32),
            Vector4::new(-0.625, 0.125, -0.375, 2081609700000000.0),
            Vector4::new(-0.625, 0.375, -0.375, 1.06485316e-16),
            Vector4::new(-0.625, 0.625, -0.375, 3.8556036e33),
            Vector4::new(-0.625, -0.125, -0.625, -4.2996805e-37),
            Vector4::new(-0.625, 0.125, -0.625, -1.1807508e-17),
            Vector4::new(-0.625, 0.375, -0.625, -1.8163785e-33),
            Vector4::new(-0.625, 0.625, -0.625, -4.0855815e17),
            Vector4::new(-0.625, -0.125, -0.875, -1.7577723e-29),
            Vector4::new(-0.625, 0.125, -0.875, -1.2098953e22),
            Vector4::new(-0.625, 0.375, -0.875, -6.238558e-20),
            Vector4::new(-0.625, 0.625, -0.875, 11.706325),
            Vector4::new(-0.375, -0.125, -0.375, 22.699986),
            Vector4::new(-0.375, 0.125, -0.375, 0.0134391785),
            Vector4::new(-0.375, 0.375, -0.375, -46.48552),
            Vector4::new(-0.375, 0.625, -0.375, 2.2352723e29),
            Vector4::new(-0.375, -0.125, -0.625, -5.4733302e-8),
            Vector4::new(-0.375, 0.125, -0.625, -19094.938),
            Vector4::new(-0.375, 0.375, -0.625, 9226.837),
            Vector4::new(-0.375, 0.625, -0.625, 0.015577154),
            Vector4::new(-0.375, -0.125, -0.875, -77.83601),
            Vector4::new(-0.375, 0.125, -0.875, 2.2474052e-26),
            Vector4::new(-0.375, 0.375, -0.875, 2.4575922e23),
            Vector4::new(-0.375, 0.625, -0.875, 5.193709e-10),
            Vector4::new(-0.875, -0.125, 0.625, -54.390636),
            Vector4::new(-0.875, 0.125, 0.625, -1.1080398e23),
            Vector4::new(-0.875, 0.375, 0.625, -2.4282049e-11),
            Vector4::new(-0.875, 0.625, 0.625, -5.552528e-14),
            Vector4::new(-0.875, -0.125, 0.375, -190106.0),
            Vector4::new(-0.875, 0.125, 0.375, 4.6039417e37),
            Vector4::new(-0.875, 0.375, 0.375, -4239.1895),
            Vector4::new(-0.875, 0.625, 0.375, 4.447598e-14),
            Vector4::new(-0.875, -0.125, 0.125, -2801645300.0),
            Vector4::new(-0.875, 0.125, 0.125, 2.0181002e38),
            Vector4::new(-0.875, 0.375, 0.125, -855.9318),
            Vector4::new(-0.875, 0.625, 0.125, 7.284196e-28),
            Vector4::new(-0.875, -0.125, -0.125, -2.6302373e16),
            Vector4::new(-0.875, 0.125, -0.125, -1.1364356e-5),
            Vector4::new(-0.875, 0.375, -0.125, 1.1231181e-12),
            Vector4::new(-0.875, 0.625, -0.125, 3438893300000.0),
            Vector4::new(-0.625, -0.125, 0.625, -1.6352307e19),
            Vector4::new(-0.625, 0.125, 0.625, -7.1314056e-37),
            Vector4::new(-0.625, 0.375, 0.625, 782555200.0),
            Vector4::new(-0.625, 0.625, 0.625, 2.2503638e35),
            Vector4::new(-0.625, -0.125, 0.375, 7.4307228e22),
            Vector4::new(-0.625, 0.125, 0.375, 1.0560371e-9),
            Vector4::new(-0.625, 0.375, 0.375, -0.6362196),
            Vector4::new(-0.625, 0.625, 0.375, -7.3897745e19),
            Vector4::new(-0.625, -0.125, 0.125, -3.435351e-27),
            Vector4::new(-0.625, 0.125, 0.125, 6.369088e36),
            Vector4::new(-0.625, 0.375, 0.125, 154.90254),
            Vector4::new(-0.625, 0.625, 0.125, 1.2365698e31),
            Vector4::new(-0.625, -0.125, -0.125, 51.762306),
            Vector4::new(-0.625, 0.125, -0.125, 2.9813502e-10),
            Vector4::new(-0.625, 0.375, -0.125, -0.012295901),
            Vector4::new(-0.625, 0.625, -0.125, 2.69508e-15),
            Vector4::new(-0.375, -0.125, 0.625, -3.832979e-22),
            Vector4::new(-0.375, 0.125, 0.625, -345.49176),
            Vector4::new(-0.375, 0.375, 0.625, -8.303447e-19),
            Vector4::new(-0.375, 0.625, 0.625, -1.3289397e-31),
            Vector4::new(-0.375, -0.125, 0.375, -2.3878333e30),
            Vector4::new(-0.375, 0.125, 0.375, -2.1728088e-17),
            Vector4::new(-0.375, 0.375, 0.375, 5.3265956e-34),
            Vector4::new(-0.375, 0.625, 0.375, 4.4693444e-32),
            Vector4::new(-0.375, -0.125, 0.125, 7.5911765e31),
            Vector4::new(-0.375, 0.125, 0.125, 2.2851525e36),
            Vector4::new(-0.375, 0.375, 0.125, 131046920000000.0),
            Vector4::new(-0.375, 0.625, 0.125, -3.3075162e-35),
            Vector4::new(-0.375, -0.125, -0.125, -6.4083335e31),
            Vector4::new(-0.375, 0.125, -0.125, 4.3839694e-20),
            Vector4::new(-0.375, 0.375, -0.125, 1.2944581e-21),
            Vector4::new(-0.375, 0.625, -0.125, -1.5476186e-5),
            Vector4::new(-0.875, -0.125, 0.875, 6.685839e-19),
            Vector4::new(-0.875, 0.125, 0.875, 2.7401933e17),
            Vector4::new(-0.875, 0.375, 0.875, 7.243506e29),
            Vector4::new(-0.875, 0.625, 0.875, 55475780000.0),
            Vector4::new(-0.625, -0.125, 0.875, -989426240.0),
            Vector4::new(-0.625, 0.125, 0.875, 9.6908307e-32),
            Vector4::new(-0.625, 0.375, 0.875, 8602509000000.0),
            Vector4::new(-0.625, 0.625, 0.875, 2.9831786e-12),
            Vector4::new(-0.375, -0.125, 0.875, -7.178455e-38),
            Vector4::new(-0.375, 0.125, 0.875, -3.127509e-20),
            Vector4::new(-0.375, 0.375, 0.875, -840335200.0),
            Vector4::new(-0.375, 0.625, 0.875, 402546240.0),
            Vector4::new(-0.875, 0.875, -0.375, -1.5210388e-18),
            Vector4::new(-0.875, 0.875, -0.625, 4.7511006e-11),
            Vector4::new(-0.875, 0.875, -0.875, -26536.994),
            Vector4::new(-0.625, 0.875, -0.375, 34080217000.0),
            Vector4::new(-0.625, 0.875, -0.625, -5710861.5),
            Vector4::new(-0.625, 0.875, -0.875, 461124.88),
            Vector4::new(-0.375, 0.875, -0.375, -1.2315972e28),
            Vector4::new(-0.375, 0.875, -0.625, -4331374.0),
            Vector4::new(-0.375, 0.875, -0.875, -4.9007916),
            Vector4::new(-0.875, 0.875, 0.625, 1.17242894e24),
            Vector4::new(-0.875, 0.875, 0.375, 1.1551313e-30),
            Vector4::new(-0.875, 0.875, 0.125, -2.2709378e35),
            Vector4::new(-0.875, 0.875, -0.125, 1.8707433e-31),
            Vector4::new(-0.625, 0.875, 0.625, 1.256856e21),
            Vector4::new(-0.625, 0.875, 0.375, 6.3084566e37),
            Vector4::new(-0.625, 0.875, 0.125, -5.5403343e25),
            Vector4::new(-0.625, 0.875, -0.125, 1.4084704e38),
            Vector4::new(-0.375, 0.875, 0.625, -1.599365e30),
            Vector4::new(-0.375, 0.875, 0.375, 1.3420701e22),
            Vector4::new(-0.375, 0.875, 0.125, 2.2141828e-33),
            Vector4::new(-0.375, 0.875, -0.125, -5.082045e-13),
            Vector4::new(-0.875, 0.875, 0.875, 1.0585613e-10),
            Vector4::new(-0.625, 0.875, 0.875, 8.4331624e32),
            Vector4::new(-0.375, 0.875, 0.875, -4.9053113e27),
            Vector4::new(-0.125, -0.875, 0.625, 6.3355777e-13),
            Vector4::new(-0.125, -0.625, 0.625, -6.911683e-38),
            Vector4::new(-0.125, -0.375, 0.625, 0.00042562114),
            Vector4::new(-0.125, -0.875, 0.875, 3.8805558e-7),
            Vector4::new(-0.125, -0.625, 0.875, -3.1603815e-17),
            Vector4::new(-0.125, -0.375, 0.875, -1.8393011e-22),
            Vector4::new(-0.125, -0.125, 0.875, 4.0982716e36),
            Vector4::new(-0.125, 0.125, 0.875, -3.4307474e-36),
            Vector4::new(-0.125, 0.375, 0.875, 0.00015133101),
            Vector4::new(-0.125, 0.625, 0.875, 4.5909157e-24),
            Vector4::new(-0.125, 0.875, 0.875, 8.9926404e-20),
        ],
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
