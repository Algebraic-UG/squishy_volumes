// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, SVD, U3, stack};

use crate::test_data::test_position_gradients_random;

use super::*;

fn check(matrices: &[Matrix4x3<f32>]) {
    let gpu_svds = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        matrices,
    );

    let cpu_svds: Vec<SVD<f32, U3, U3>> = matrices
        .iter()
        .map(|m| {
            let m: Matrix3<f32> = m.fixed_view::<3, 3>(0, 0).into();
            SVD::new(m, true, true)
        })
        .collect();

    for (cpu, gpu) in cpu_svds.into_iter().zip(gpu_svds) {
        println!("u");
        check_iters_by_norm(&cpu.u.unwrap(), gpu.u.fixed_view::<3, 3>(0, 0));
        println!("s");
        check_iters_by_norm(&cpu.singular_values, &gpu.s.xyz());
        println!("v");
        check_iters_by_norm(&cpu.v_t.unwrap(), gpu.v.fixed_view::<3, 3>(0, 0));
    }
}

#[test]
fn random() {
    check(
        #[allow(clippy::toplevel_ref_arg)]
        &test_position_gradients_random(1000)
            .into_iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect::<Vec<_>>(),
    );
}

fn run(settings: Settings, matrices: &[Matrix4x3<f32>]) -> Vec<Svd> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), matrices).unwrap();

    let test_svd = TestSvd::new(&context, settings);
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { svds } = test_svd
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, svds);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
