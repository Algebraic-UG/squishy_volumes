// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::repeat;

use super::*;

fn check(workgroup_size: u32, dispatch_limit: u32, cells_in: &[Vector4<i32>], limit: u32) {
    let limits = [limit, 0, 0, 0, 0, 0, 0, 0];
    let indirect = find_x_y_z_simple(dispatch_limit, limit.div_ceil(workgroup_size))
        .into_iter()
        .chain(repeat(0))
        .take(8 * 3)
        .collect::<Vec<_>>();
    println!("limit: {limits:?}");
    println!("indirect: {indirect:?}");

    {
        let tmp = cells_in.iter().take(limit as usize);
        let counts = (0..8)
            .map(|color| {
                tmp.clone()
                    .filter(|cell| {
                        let cell = cell.map(|c| i32_to_u32_offset(c) & 1);
                        cell.x | (cell.y << 1) | (cell.z << 2) == color
                    })
                    .count() as u32
            })
            .collect::<Vec<_>>();
        println!("counts: {counts:?}");
        let limits: Vec<_> = counts
            .iter()
            .scan(0, |prefix_sum, item| {
                *prefix_sum += item;
                Some(*prefix_sum)
            })
            .collect();
        println!("limits: {limits:?}");
        let indirect: Vec<_> = counts
            .iter()
            .map(|count| find_x_y_z_simple(limit, count.div_ceil(workgroup_size)))
            .collect();
        println!("indirect: {indirect:?}");
    }

    let (limits, indirect, cells_out) =
        run_color_cells_2(workgroup_size, dispatch_limit, &limits, &indirect, cells_in);

    println!("(GPU) limit: {limits:?}");
    println!("(GPU) indirect: {indirect:?}");

    let mut start = 0;
    for color in 0..8 {
        let cell = cells_out[start as usize];

        println!("now checking: {:?}", cell.map(|c| i32_to_u32_offset(c) & 1));

        let end = limits[color];
        let count = end - start;

        assert!(
            indirect[color * 3..(color + 1) * 3].iter().product::<u32>()
                >= count.div_ceil(workgroup_size)
        );

        for index in start..end {
            println!("{start} {index} {end}");
            assert_eq!(
                cell.map(|c| i32_to_u32_offset(c) & 1),
                cells_out[index as usize].map(|c| i32_to_u32_offset(c) & 1),
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
        workgroup_size,
        dispatch_limit,
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
        8,
    );
}

#[test]
fn test_simple_ignore_half() {
    let workgroup_size = 64;
    let dispatch_limit = 10;
    check(
        workgroup_size,
        dispatch_limit,
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
        4,
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

    check(workgroup_size, dispatch_limit, &cells, cells.len() as u32);
}

#[test]
fn test_random_ignore_half() {
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
        workgroup_size,
        dispatch_limit,
        &cells,
        cells.len() as u32 / 2,
    );
}

fn run_color_cells_2(
    workgroup_size: u32,
    dispatch_limit: u32,
    limits: &[u32],
    indirect: &[u32],
    cells: &[Vector4<i32>],
) -> (Vec<u32>, Vec<u32>, Vec<Vector4<i32>>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let color_cells_2 = ColorCells2::new(
        &context,
        ColorCells2Settings {
            workgroup_size,
            dispatch_limit,
        },
    );
    let buffers = color_cells_2.create_buffers(
        &context,
        ColorCells2BufferInput {
            cells,
            limits,
            indirect,
        },
    );

    let downloads = DownloadsToHost::new(
        &context,
        [
            (&buffers.limits, "limits"),
            (&buffers.indirect, "indirect"),
            (&buffers.counts, "counts"),
            (&buffers.prefix_sums, "prefix_sums"),
            (&buffers.cells_out, "cells_out"),
        ],
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    color_cells_2.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let dowloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [limits, indirect, counts, prefix_sums, cells_out] = dowloads.try_into().unwrap();

    println!("(GPU) counts {:?}", counts.to_vec::<u32>());
    println!("(GPU) prefix_sums {:?}", prefix_sums.to_vec::<u32>());

    (limits.to_vec(), indirect.to_vec(), cells_out.to_vec())
}
