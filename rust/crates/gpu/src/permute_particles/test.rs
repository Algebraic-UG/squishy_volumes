// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(positions: Vec<Vector4<f32>>) {
    let mut indices: Vec<_> = (0..positions.len() as u32).collect();
    let mut permutation = indices.clone();

    shuffle(&mut indices, 7);
    shuffle(&mut permutation, 5);

    println!("indices: {indices:?}");
    println!("permutation: {permutation:?}");

    let mut permuted_indices = indices.clone();
    for (&prior_position, to_permute) in permutation.iter().zip(&mut permuted_indices) {
        *to_permute = indices[prior_position as usize];
    }

    let mut permuted_postions = positions.clone();
    for (&prior_position, to_permute) in permutation.iter().zip(&mut permuted_postions) {
        *to_permute = positions[prior_position as usize];
    }

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    let (indices_out, positions_out) = run_permute_particles(
        workgroup_size,
        dispatch_limit,
        &permutation,
        &indices,
        &positions,
    );

    assert_eq!(permuted_indices, indices_out);
    assert_eq!(permuted_postions, positions_out);
}

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
    check(positions);
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
    check(positions);
}

fn run_permute_particles(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    permutation: &[u32],
    indices: &[u32],
    positions: &[Vector4<f32>],
) -> (Vec<u32>, Vec<Vector4<f32>>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        workgroup_size,
        dispatch_limit,
        permutation,
        indices,
        positions,
    );
    let permute_particles = PermuteParticles::new(&context, Settings { workgroup_size });

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indices_out,
        positions_out,
    } = permute_particles
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [indices_out, positions_out]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [indices_out, positions_out] = downloads.try_into().unwrap();

    let mut garbage_w: Vec<Vector4<f32>> = positions_out.to_vec();
    garbage_w.iter_mut().for_each(|v| v.w = 0.);

    (indices_out.to_vec(), garbage_w)
}
