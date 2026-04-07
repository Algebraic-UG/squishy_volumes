// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(cells: &[Vector4<i32>]) {
    let colorkeys = cells_to_colorkeys_on_cpu(cells);

    let (limits, indirect, indices) = run_color_cells(64, cells);

    let mut start = 0;
    for color in 0..8 {
        let colorkey = colorkeys[indices[start as usize] as usize];

        let end = limits[color];
        let count = end - start;

        assert!(indirect[color * 3..(color + 1) * 3].iter().product::<u32>() >= count);

        for index in &indices[start as usize..end as usize] {
            assert_eq!(colorkey, colorkeys[*index as usize]);
        }

        start = end;
    }
}

#[test]
fn test_simple() {
    check(&[
        Vector4::new(-5, -5, -5, 0),
        Vector4::new(-5, -5, 5, 0),
        Vector4::new(-5, 5, -5, 0),
        Vector4::new(-5, 5, 5, 0),
        Vector4::new(5, -5, -5, 0),
        Vector4::new(5, -5, 5, 0),
        Vector4::new(5, 5, -5, 0),
        Vector4::new(5, 5, 5, 0),
    ]);
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cells: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(1000 * 4)
        .collect();
    let cells: Vec<Vector4<i32>> = cells
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    check(&cells);
}

fn run_color_cells(workgroup_size: u32, cells: &[Vector4<i32>]) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let color_cells = ColorCells::new(&context, ColorCellsSettings { workgroup_size });
    let buffers = color_cells.create_buffers(&context, ColorCellsBufferInput { cells });

    let downloads = DownloadsToHost::new(
        &context,
        [
            (&buffers.limits, "limits"),
            (&buffers.indirect, "indirect"),
            (&buffers.radix_sort.indices_back, "indices"),
        ],
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    color_cells.compute_in_pass(&context, &mut compute_pass, &mut (&buffers).into(), &mut ());

    drop(compute_pass);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let dowloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [limits, indirect, indices] = dowloads.try_into().unwrap();

    (limits.to_vec(), indirect.to_vec(), indices.to_vec())
}
