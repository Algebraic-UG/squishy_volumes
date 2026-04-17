// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

#[test]
fn test_simple() {
    let cell_size = 0.3;

    let positions = [
        Vector4::new(0., 0., 0., 0.),
        Vector4::new(0.1, 0., 0., 0.),
        Vector4::new(0.2, 0., 0., 0.),
        Vector4::new(0.3, 0., 0., 0.),
        Vector4::new(0.4, 0., 0., 0.),
        Vector4::new(0.5, 0., 0., 0.),
        Vector4::new(0.6, 0., 0., 0.),
        Vector4::new(0.7, 0., 0., 0.),
    ];

    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        cell_size,
    };

    assert_eq!(
        vec![0, 0, 1, 0, 0, 1, 0, 1],
        run_find_cell_boundaries(settings, dispatch_limit, &positions)
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

    let sorted_indices = sort_positions_into_cells_on_cpu(
        &(0..positions.len() as u32).collect::<Vec<_>>(),
        &positions,
        cell_size,
    );

    let mut positions = positions;
    let lookup = positions.clone();
    sorted_indices
        .iter()
        .zip(&mut positions)
        .for_each(|(index, position)| *position = lookup[*index as usize]);

    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        cell_size,
    };

    assert_eq!(
        find_cell_boundaries_on_cpu(&positions, cell_size),
        run_find_cell_boundaries(settings, dispatch_limit, &positions)
    )
}

fn run_find_cell_boundaries(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        positions,
    );
    let find_cell_boundaries = FindCellBoundaries::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { boundaries } = find_cell_boundaries
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, boundaries);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
