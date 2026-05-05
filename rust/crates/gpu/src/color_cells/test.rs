// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(workgroup_size: NonZeroU32, dispatch_limit: NonZeroU32, cells: &[Vector4<i32>]) {
    let subgroup_size = get_subgroup_size();

    let settings = Settings {
        workgroup_size,
        dispatch_limit,
    };

    let (indirect_colors, indirect_colors_batch, indices) =
        color_cells_on_cpu(workgroup_size, dispatch_limit, subgroup_size, cells);

    let (gpu_indirect_colors, gpu_indirect_colors_batch, gpu_indices) =
        run_color_cells(settings, cells);
    assert_eq!(indirect_colors, gpu_indirect_colors);
    assert_eq!(gpu_indirect_colors_batch, indirect_colors_batch);
    assert_eq!(indices, gpu_indices);

    let mut start: u32 = 0;
    for indirect_color in indirect_colors {
        if start == indices.len() as u32 {
            assert_eq!(start, indirect_color.len);
            assert_eq!(0, indirect_color.x);
            assert_eq!(0, indirect_color.y);
            assert_eq!(0, indirect_color.z);
            continue;
        }

        let index = indices[start as usize];
        let cell = cells[index as usize];

        println!("now checking: {:?}", cell.map(|c| i32_to_u32_offset(c) & 1));

        let end = indirect_color.len;
        for index in start..end {
            println!("{start} {index} {end}");
            assert_eq!(
                cell.map(|c| i32_to_u32_offset(c) & 1),
                cells[indices[index as usize] as usize].map(|c| i32_to_u32_offset(c) & 1),
            );
        }

        println!("checked {start} to {end}");
        start = end;
    }
}

#[test]
fn test_simple() {
    let workgroup_size = 64;
    let dispatch_limit = 10;
    check(
        workgroup_size.try_into().unwrap(),
        dispatch_limit.try_into().unwrap(),
        &[
            Vector4::new(-5, -5, -5, 0),
            Vector4::new(-5, -5, 5, 0),
            Vector4::new(-5, 5, -5, 0),
            Vector4::new(-5, 5, 5, 0),
            Vector4::new(5, -5, -5, 0),
            Vector4::new(5, -5, 5, 0),
            Vector4::new(5, 5, -5, 0),
            Vector4::new(5, 5, 5, 0),
        ],
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let workgroup_size = 64;
    let dispatch_limit = 10;

    let cells: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(1000 * 4)
        .collect();
    let cells: Vec<Vector4<i32>> = cells
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .map(|cell| cell.xyz().push(0))
        .collect();

    check(
        workgroup_size.try_into().unwrap(),
        dispatch_limit.try_into().unwrap(),
        &cells,
    );
}

fn run_color_cells(
    settings: Settings,
    cells: &[Vector4<i32>],
) -> (Vec<Indirect>, Vec<Indirect>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let color_cells = ColorCells::new(&context, settings);
    let input = Input::new(context.device(), settings, cells);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indirect_colors,
        indirect_colors_batch,
        indices,
    } = color_cells
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads =
        DownloadsToHost::new(&context, [indirect_colors, indirect_colors_batch, indices]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let dowloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [indirect_colors, indirect_colors_batch, indices] = dowloads.try_into().unwrap();

    (
        indirect_colors.to_vec(),
        indirect_colors_batch.to_vec(),
        indices.to_vec(),
    )
}
