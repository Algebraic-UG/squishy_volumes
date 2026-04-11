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
    let count = 100;

    assert_eq!(
        (0..100).collect::<Vec<u32>>(),
        run_generate_indices(64, count),
    );
}
fn run_generate_indices(workgroup_size: u32, count: u32) -> Vec<u32> {
    let context = SHARED_CONTEXT.lock().unwrap();
    let device = context.device();

    let generate_indices =
        GenerateIndices::new(&context, GenerateIndicesSettings { workgroup_size });
    let buffers =
        generate_indices.create_buffers(&context, GenerateIndicesBufferInput { count: &count });

    let download = DownloadToHost::new(&context, &buffers.indices, "indices");

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    generate_indices.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());

    drop(compute_pass);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    download.to_vec()
}
