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

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, stack};
use squishy_volumes_util::fluid_plasticity;

use crate::test_data::test_position_gradients_random;

use super::*;

fn check(particle_position_gradients: &[Matrix4x3<f32>]) {
    let gpu_particle_position_gradients = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        particle_position_gradients,
    );

    let mut cpu_particle_position_gradients: Vec<Matrix3<f32>> = particle_position_gradients
        .iter()
        .map(|m| m.fixed_view::<3, 3>(0, 0).into())
        .collect();

    cpu_particle_position_gradients
        .iter_mut()
        .for_each(|position_gradient| *position_gradient = fluid_plasticity(position_gradient));

    for (cpu, gpu) in cpu_particle_position_gradients
        .into_iter()
        .zip(gpu_particle_position_gradients)
    {
        println!("{} vs {}", cpu, gpu.fixed_view::<3, 3>(0, 0));

        check_iters_by_norm(&cpu, gpu.fixed_view::<3, 3>(0, 0));
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

fn run(settings: Settings, particle_position_gradients: &[Matrix4x3<f32>]) -> Vec<Matrix4x3<f32>> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), particle_position_gradients).unwrap();
    let particle_position_gradients = input.particle_position_gradients.clone();

    let fluid = Fluid::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = fluid
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, particle_position_gradients);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
