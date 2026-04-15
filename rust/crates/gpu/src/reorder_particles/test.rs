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
    let workgroup_size = 64;
    let limits = [3];
    let indirect = [1, 1, 1];
    let cell_indices = [2, 1, 0];
    let index_ranges = [3, 6, 8];
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

    let (offsets, reordered_positions) = run_reorder_particles(
        workgroup_size,
        ReorderParticlesBufferInput {
            limits: &limits,
            indirect: &indirect,
            cell_indices: &cell_indices,
            index_ranges: &index_ranges,
            positions: &positions,
        },
    );

    assert_eq!(offsets, [0, 2, 5]);
    assert_eq!(
        reordered_positions,
        [
            Vector4::new(0.5, 0.5, -0.5, 0.),
            Vector4::new(0.5, 0.5, 0.5, 0.),
            Vector4::new(-0.5, 0.5, 0.5, 0.),
            Vector4::new(0.5, -0.5, -0.5, 0.),
            Vector4::new(0.5, -0.5, 0.5, 0.),
            Vector4::new(-0.5, -0.5, -0.5, 0.),
            Vector4::new(-0.5, -0.5, 0.5, 0.),
            Vector4::new(-0.5, 0.5, -0.5, 0.),
        ]
    );
}

fn run_reorder_particles(
    workgroup_size: u32,
    buffer_input: ReorderParticlesBufferInput,
) -> (Vec<u32>, Vec<Vector4<f32>>) {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let reorder_particles = ReorderParticles::new(
        &context,
        ReorderParticlesSettings {
            workgroup_size,
            prefix_sum: PrefixSumSettings { workgroup_size },
        },
    );
    let buffers = reorder_particles.create_buffers(&context, buffer_input);

    let downloads = DownloadsToHost::new(
        &context,
        [
            (&buffers.counts, "counts"),
            (&buffers.offsets, "offsets"),
            (&buffers.positions_out, "positions_out"),
        ],
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    reorder_particles.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let dowloads = downloads.prep();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let [counts, offsets, positions] = dowloads.try_into().unwrap();

    println!("{:?}", counts.to_vec::<u32>());

    (offsets.to_vec(), positions.to_vec())
}
