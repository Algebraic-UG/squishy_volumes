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

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    assert_eq!(
        permuted_postions,
        run_permute_positions(workgroup_size, dispatch_limit, &permutation, &positions),
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

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    assert_eq!(
        permuted_postions,
        run_permute_positions(workgroup_size, dispatch_limit, &permutation, &positions),
    );
}

fn run_permute_positions(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    permutation: &[u32],
    positions: &[Vector4<f32>],
) -> Vec<Vector4<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        workgroup_size,
        dispatch_limit,
        permutation,
        positions,
    );
    let permute_positions = PermutePositions::new(&context, Settings { workgroup_size });

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { positions_out } = permute_positions
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, positions_out);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let mut garbage_w: Vec<Vector4<f32>> = download.to_vec();
    garbage_w.iter_mut().for_each(|v| v.w = 0.);
    garbage_w
}
