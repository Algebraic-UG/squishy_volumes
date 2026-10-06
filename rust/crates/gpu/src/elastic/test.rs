// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::{repeat, repeat_n};

use approx::assert_relative_eq;
use nalgebra::{Matrix1x3, Matrix3, stack};
use rand::{SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_util::{
    elastic_energy_inviscid, first_piola_stress_inviscid, first_piola_stress_neo_hookean,
    try_elastic_energy_neo_hookean,
};

use crate::test_data::{
    test_inviscid_parameters, test_lame_parameters, test_position_gradients_random,
};

use super::*;

fn check(
    position_gradients: &[Matrix3<f32>],
    flags: &[ParticleFlags],
    parameters: &[ParticleParameters],
) {
    let particle_flags: Vec<_> = flags
        .iter()
        .cloned()
        .flat_map(|p| repeat_n(p, position_gradients.len()))
        .collect();
    let particle_parameters: Vec<_> = parameters
        .iter()
        .cloned()
        .flat_map(|p| repeat_n(p, position_gradients.len()))
        .collect();

    let position_gradients_host = position_gradients.repeat(parameters.len());
    assert_eq!(particle_parameters.len(), position_gradients_host.len());
    assert_eq!(particle_flags.len(), position_gradients_host.len());

    #[allow(clippy::toplevel_ref_arg)]
    let position_gradients_device = position_gradients_host
        .iter()
        .map(|m| {
            stack![
                m;
                Matrix1x3::zeros()
            ]
        })
        .collect::<Vec<_>>();

    let (stresses_cpu, energies_cpu): (Vec<Matrix3<f32>>, Vec<f32>) = position_gradients_host
        .iter()
        .zip(&particle_flags)
        .zip(&particle_parameters)
        .map(|((position_gradient, flags), parameters)| {
            if flags.contains(ParticleFlags::IS_SOLID) {
                (
                    first_piola_stress_neo_hookean(
                        parameters.mu(),
                        parameters.lambda(),
                        position_gradient,
                    ),
                    try_elastic_energy_neo_hookean(
                        parameters.mu(),
                        parameters.lambda(),
                        position_gradient,
                    )
                    .unwrap(),
                )
            } else if flags.contains(ParticleFlags::IS_FLUID) {
                (
                    first_piola_stress_inviscid(
                        parameters.bulk_modulus,
                        parameters.exponent,
                        position_gradient,
                    ),
                    elastic_energy_inviscid(
                        parameters.bulk_modulus,
                        parameters.exponent,
                        position_gradient,
                    ),
                )
            } else {
                unreachable!()
            }
        })
        .unzip();

    let (stresses_gpu, energies_gpu) = run_elastic(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
        },
        (u16::MAX as u32).try_into().unwrap(),
        &position_gradients_device,
        &particle_flags,
        &particle_parameters,
    );

    for i in 0..position_gradients_host.len() {
        println!("position_gradient: {:?}", position_gradients_host[i]);
        println!("Parameters: {:?}", particle_parameters[i]);
        println!("cpu: {}, {:?}", energies_cpu[i], stresses_cpu[i]);
        println!("gpu: {}, {:?}", energies_gpu[i], stresses_gpu[i]);

        check_iters(
            stresses_cpu[i].iter(),
            stresses_gpu[i].fixed_view::<3, 3>(0, 0).iter(),
        );
        assert_relative_eq!(energies_cpu[i], energies_gpu[i], max_relative = 0.01);
    }
}

#[test]
fn solid_simple() {
    let mut rng = ChaCha8Rng::seed_from_u64(40);
    let parameters = test_lame_parameters(&mut rng).collect::<Vec<_>>();
    check(
        &test_position_gradients_simple(),
        &vec![ParticleFlags::IS_SOLID; parameters.len()],
        &parameters,
    );
}

#[test]
fn solid_random() {
    let mut rng = ChaCha8Rng::seed_from_u64(41);
    let parameters = test_lame_parameters(&mut rng).collect::<Vec<_>>();
    check(
        &test_position_gradients_random(100),
        &vec![ParticleFlags::IS_SOLID; parameters.len()],
        &parameters,
    );
}

#[test]
fn fluid_simple() {
    let mut rng = ChaCha8Rng::seed_from_u64(41);
    let parameters = test_lame_parameters(&mut rng).collect::<Vec<_>>();
    check(
        &test_position_gradients_simple(),
        &vec![ParticleFlags::IS_FLUID; parameters.len()],
        &parameters,
    );
}

#[test]
fn fluid_random() {
    let mut rng = ChaCha8Rng::seed_from_u64(43);
    let parameters = test_lame_parameters(&mut rng).collect::<Vec<_>>();
    check(
        &test_position_gradients_random(100),
        &vec![ParticleFlags::IS_FLUID; parameters.len()],
        &parameters,
    );
}

#[test]
fn mixed_random() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let n = 100;
    let (particle_parameters, particle_flags): (Vec<_>, Vec<_>) = test_lame_parameters(&mut rng)
        .zip(repeat(ParticleFlags::IS_SOLID))
        .collect::<Vec<_>>()
        .into_iter()
        .chain((test_inviscid_parameters(&mut rng)).zip(repeat(ParticleFlags::IS_FLUID)))
        .unzip();

    check(
        &test_position_gradients_random(n),
        &particle_flags,
        &particle_parameters,
    );
}

fn run_elastic(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    position_gradients: &[Matrix4x3<f32>],
    particle_flags: &[ParticleFlags],
    particle_parameters: &[ParticleParameters],
) -> (Vec<Matrix4x3<f32>>, Vec<f32>) {
    let mut context = get_shared_context();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        position_gradients,
        particle_flags,
        particle_parameters,
    )
    .unwrap();

    let elastic = Elastic::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { stresses, energies } = elastic
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [stresses, energies]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [stresses, energies] = downloads.try_into().unwrap();

    (stresses.to_vec().unwrap(), energies.to_vec().unwrap())
}
