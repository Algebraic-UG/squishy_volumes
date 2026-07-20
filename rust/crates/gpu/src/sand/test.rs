// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector3, stack};
use squishy_volumes_file_frame::SpecificParticleParameters;

use crate::test_data::test_position_gradients_random;

use super::*;

fn check(
    particle_parameters: &[ParticleParameters],
    particle_position_gradients: &[Matrix4x3<f32>],
) {
    let particle_flags: Vec<ParticleFlags> = particle_parameters.iter().map(Into::into).collect();

    let gpu_particle_position_gradients = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        &particle_flags,
        &particle_parameters,
        particle_position_gradients,
    );

    let mut cpu_particle_position_gradients: Vec<Matrix3<f32>> = particle_position_gradients
        .iter()
        .map(|m| m.fixed_view::<3, 3>(0, 0).into())
        .collect();

    particle_parameters
        .iter()
        .zip(&mut cpu_particle_position_gradients)
        .for_each(|(parameters, position_gradient)| {
            let SpecificParticleParameters::Solid {
                mu,
                lambda,
                sand_alpha: Some(sand_alpha),
            } = parameters.specific
            else {
                return;
            };

            let mut svd = position_gradient.svd(true, true);
            let e = svd.singular_values.map(f32::ln);
            let e_tr = e.sum();
            let e_hat = e - Vector3::repeat(e_tr / 3.);
            let e_hat_norm = e_hat.norm();
            if mu > 0. && e_tr < 0. && e_hat_norm > 0. {
                if e_hat_norm != 0. {
                    let delta_gamma =
                        e_hat_norm + (3. * lambda + 2. * mu) / 2. / mu * e_tr * sand_alpha;
                    if delta_gamma > 0. {
                        let big_h = e - delta_gamma / e_hat_norm * e_hat;
                        svd.singular_values = big_h.map(f32::exp);

                        *position_gradient = svd.recompose().unwrap();
                    }
                }
            } else {
                *position_gradient = svd.u.unwrap() * svd.v_t.unwrap();
            }
        });

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
    let n = 1000;
    check(
        &squishy_volumes_util::test_lame_parameters()
            .cycle()
            .take(n)
            .map(|[mu, lambda]| ParticleParameters {
                mass: 1.,
                initial_volume: 1.,
                viscosity: None,
                specific: SpecificParticleParameters::Solid {
                    mu,
                    lambda,
                    sand_alpha: Some(0.3),
                },
            })
            .collect::<Vec<_>>(),
        #[allow(clippy::toplevel_ref_arg)]
        &test_position_gradients_random(1000)
            .into_iter()
            .map(|m| stack![m; Matrix1x3::zeros()])
            .collect::<Vec<_>>(),
    );
}

fn run(
    settings: Settings,
    particle_flags: &[ParticleFlags],
    particle_parameters: &[ParticleParameters],
    particle_position_gradients: &[Matrix4x3<f32>],
) -> Vec<Matrix4x3<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        particle_flags,
        particle_parameters,
        particle_position_gradients,
    )
    .unwrap();
    let particle_position_gradients = input.particle_position_gradients.clone();

    let sand = Sand::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = sand
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
