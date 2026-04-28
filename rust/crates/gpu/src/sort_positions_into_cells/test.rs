// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector4;

use super::*;

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
    let indices = (0..positions.len() as u32).collect::<Vec<_>>();

    let cell_size = 1.;
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        cell_size,
        bit_count: 2.try_into().unwrap(),
    };

    assert_eq!(
        sort_positions_into_cells_on_cpu(&indices, &positions, cell_size),
        run_sort_positions_into_cells(settings, &positions),
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
    let indices: Vec<_> = (0..positions.len() as u32).collect();

    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        cell_size,
        bit_count: 2.try_into().unwrap(),
    };

    assert_eq!(
        sort_positions_into_cells_on_cpu(&indices, &positions, cell_size),
        run_sort_positions_into_cells(settings, &positions),
    );
}

fn run_sort_positions_into_cells(settings: Settings, positions: &[Vector4<f32>]) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, positions);
    let sort_positions_into_cells = SortPositionsIntoCells::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { permutation } = sort_positions_into_cells
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, permutation);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);
    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
