// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(positions: &[Vector4<f32>], prefixed_boundaries: &[u32], cell_size: f32) {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    let (mut cells, mut index_ranges, new_indirect) = run_build_cells(
        Settings {
            workgroup_size,
            dispatch_limit,
            cell_size,
        },
        positions,
        prefixed_boundaries,
    );

    let mut index_start = 0;
    for (index_end, cell) in index_ranges
        .iter()
        .zip(cells.clone())
        .take(new_indirect.len as usize)
    {
        for index in index_start..*index_end {
            println!("{cell:?}, {index}");
            assert_eq!(
                position_to_cell(cell_size, &positions[index as usize]),
                cell
            );
        }
        index_start = index_end + 1;
    }

    cells.resize(new_indirect.len as usize, Default::default());
    index_ranges.resize(new_indirect.len as usize, Default::default());

    assert_eq!(
        build_cells_on_cpu(
            workgroup_size,
            dispatch_limit,
            cell_size,
            positions,
            prefixed_boundaries,
        ),
        (cells, index_ranges, new_indirect),
    );
}

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

    let prefixed_boundaries = prefix_sum_on_cpu(&[0, 0, 1, 0, 0, 1, 0, 1]);

    check(&positions, &prefixed_boundaries, cell_size);
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

    let cell_boundaries = find_cell_boundaries_on_cpu(&positions, cell_size);
    let prefixed_boundaries = prefix_sum_on_cpu(&cell_boundaries);

    check(&positions, &prefixed_boundaries, cell_size);
}

fn run_build_cells(
    settings: Settings,
    positions: &[Vector4<f32>],
    prefixed_boundaries: &[u32],
) -> (Vec<Vector4<i32>>, Vec<u32>, Indirect) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, positions, prefixed_boundaries);
    let build_cells = BuildCells::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        cell_ids,
        index_ranges,
        new_indirect,
        ..
    } = build_cells
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [cell_ids, index_ranges, new_indirect]);
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [cell_ids, index_ranges, new_indirect] = downloads.try_into().unwrap();
    let mut garbage_w: Vec<Vector4<i32>> = cell_ids.to_vec();
    garbage_w.iter_mut().for_each(|v| v.w = 0);

    (garbage_w, index_ranges.to_vec(), new_indirect.to_vec()[0])
}
