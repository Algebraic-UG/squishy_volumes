// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::iter::repeat;

use approx::assert_relative_eq;
use nalgebra::{Matrix1x3, Matrix3, stack};
use rand::{SeedableRng as _, rngs::ChaCha8Rng};
use squishy_volumes_file_frame::SpecificParticleParameters;
use squishy_volumes_util::{
    elastic_energy_inviscid, first_piola_stress_inviscid, first_piola_stress_neo_hookean,
    try_elastic_energy_neo_hookean,
};

use crate::test_data::{
    test_inviscid_parameters, test_lame_parameters, test_position_gradients_random,
};

use super::*;

fn check(position_gradients: &[Matrix3<f32>], parameters: &[ParticleParameters]) {
    let particle_parameters: Vec<_> = parameters
        .iter()
        .cloned()
        .flat_map(|p| repeat(p).take(position_gradients.len()))
        .collect();
    let position_gradients_host = position_gradients.repeat(parameters.len());
    assert_eq!(particle_parameters.len(), position_gradients_host.len());

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
    let particle_flags: Vec<_> = particle_parameters.iter().map(Into::into).collect();

    let (stresses_cpu, energies_cpu): (Vec<Matrix3<f32>>, Vec<f32>) = position_gradients_host
        .iter()
        .zip(particle_parameters.clone())
        .map(
            |(position_gradient, parameters)| match parameters.specific {
                SpecificParticleParameters::Solid { mu, lambda, .. } => (
                    first_piola_stress_neo_hookean(mu, lambda, position_gradient),
                    try_elastic_energy_neo_hookean(mu, lambda, position_gradient).unwrap(),
                ),
                SpecificParticleParameters::Fluid {
                    exponent,
                    bulk_modulus,
                } => (
                    first_piola_stress_inviscid(bulk_modulus, exponent, position_gradient),
                    elastic_energy_inviscid(bulk_modulus, exponent, position_gradient),
                ),
            },
        )
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
fn test_solid_simple() {
    let mut rng = ChaCha8Rng::seed_from_u64(40);
    check(
        &test_position_gradients_simple(),
        &test_lame_parameters(&mut rng).collect::<Vec<_>>(),
    );
}

#[test]
fn test_solid_random() {
    let mut rng = ChaCha8Rng::seed_from_u64(41);
    check(
        &test_position_gradients_random(100),
        &test_lame_parameters(&mut rng).collect::<Vec<_>>(),
    );
}

#[test]
fn test_fluid_simple() {
    let mut rng = ChaCha8Rng::seed_from_u64(41);
    check(
        &test_position_gradients_simple(),
        &test_inviscid_parameters(&mut rng).collect::<Vec<_>>(),
    );
}

#[test]
fn test_fluid_random() {
    let mut rng = ChaCha8Rng::seed_from_u64(43);
    check(
        &test_position_gradients_random(100),
        &test_inviscid_parameters(&mut rng).collect::<Vec<_>>(),
    );
}

#[test]
fn test_mixed_random() {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    check(
        &test_position_gradients_random(100),
        &test_lame_parameters(&mut rng)
            .collect::<Vec<_>>()
            .into_iter()
            .chain(test_inviscid_parameters(&mut rng))
            .collect::<Vec<_>>(),
    );
}

fn run_elastic(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    position_gradients: &[Matrix4x3<f32>],
    particle_flags: &[ParticleFlags],
    particle_parameters: &[ParticleParameters],
) -> (Vec<Matrix4x3<f32>>, Vec<f32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        position_gradients,
        particle_flags,
        particle_parameters,
    )
    .unwrap();

    let elastic = Elastic::new(&mut context, settings);
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

    (stresses.to_vec(), energies.to_vec())
}
