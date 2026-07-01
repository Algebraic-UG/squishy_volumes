// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

#[test]
fn read_back_status() {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let test_status = TestStatus::new(&mut context, Settings);
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = test_status
        .record(&mut context, &mut (&mut encoder).into(), Input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, context.status());
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let status: GpuStatus = download.to_vec()[0];

    context.reset_status().unwrap();

    assert_eq!(
        context.get_shader_id(test_status.test_status.label.unwrap()),
        status.shader_id()
    );
    assert!(status.table_tries_exceeded());
}
