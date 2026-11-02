// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
        println!("{} vs {}", cpu.u.unwrap(), gpu.u.fixed_view::<3, 3>(0, 0));
        println!("{} vs {}", cpu.singular_values, gpu.s.xyz());
        println!(
            "{} vs {}",
            cpu.v_t.unwrap(),
            gpu.v_t.fixed_view::<3, 3>(0, 0)
        );
        println!(
            "{} vs {}",
            cpu.recompose().unwrap(),
            gpu.u.fixed_view::<3, 3>(0, 0)
                * Matrix3::from_diagonal(&gpu.s.xyz())
                * gpu.v_t.fixed_view::<3, 3>(0, 0)
        );

        let cpu_uv = cpu.u.unwrap() * cpu.v_t.unwrap();
        let gpu_uv = gpu.u.fixed_view::<3, 3>(0, 0) * gpu.v_t.fixed_view::<3, 3>(0, 0);

        println!("uv");
        check_iters_by_norm(&cpu_uv, &gpu_uv);

        println!("singular values");
        check_iters_by_norm(&cpu.singular_values, &gpu.s.xyz());
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
    let mut context = get_shared_context();

    let input = Input::new(context.device(), matrices).unwrap();

    let test_svd = TestSvd::new(&mut context, settings).unwrap();
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

    download.to_vec().unwrap()
}
