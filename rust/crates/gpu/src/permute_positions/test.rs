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
    let positions = vec![
        Vector4::new(-0.5, -0.5, -0.5, 0.),
        Vector4::new(-0.5, -0.5, 0.5, 0.),
        Vector4::new(-0.5, 0.5, -0.5, 0.),
        Vector4::new(-0.5, 0.5, 0.5, 0.),
        Vector4::new(0.5, -0.5, -0.5, 0.),
        Vector4::new(0.5, -0.5, 0.5, 0.),
        Vector4::new(0.5, 0.5, -0.5, 0.),
        Vector4::new(0.5, 0.5, 0.5, 0.),
    ];
    let mut permutation: Vec<_> = (0..positions.len() as u32).collect();
    shuffle(&mut permutation, 5);

    let mut permuted_postions = positions.clone();
    for (&prior_position, to_permute) in permutation.iter().zip(&mut permuted_postions) {
        *to_permute = positions[prior_position as usize];
    }

    assert_eq!(
        permuted_postions,
        run_permute_positions(64, &permutation, &positions),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000 * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .map(|p| p.xzy().push(0.))
        .collect();

    let mut permutation: Vec<_> = (0..positions.len() as u32).collect();
    shuffle(&mut permutation, 5);

    let mut permuted_postions = positions.clone();
    for (&prior_position, to_permute) in permutation.iter().zip(&mut permuted_postions) {
        *to_permute = positions[prior_position as usize];
    }

    assert_eq!(
        permuted_postions,
        run_permute_positions(64, &permutation, &positions),
    );
}

fn run_permute_positions(
    workgroup_size: u32,
    permutation: &[u32],
    positions: &[Vector4<f32>],
) -> Vec<Vector4<f32>> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let permute_positions =
        PermutePositions::new(&context, PermutePositionsSettings { workgroup_size });

    let buffers = permute_positions.create_buffers(
        &context,
        PermutePositionsBufferInput {
            permutation,
            positions,
        },
    );

    let download = DownloadToHost::new(&context, &buffers.positions_out, "positions_out");

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    permute_positions.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    download.to_vec()
}
