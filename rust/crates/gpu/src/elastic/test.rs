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
use squishy_volumes_util::{
    elastic_energy_inviscid, first_piola_stress_inviscid, first_piola_stress_neo_hookean,
    try_elastic_energy_neo_hookean,
};

use crate::particle_parameters::{Fluid, Host, Solid};

use super::*;

#[test]
fn test_solid_simple() {
    let position_gradients_host = test_position_gradients_simple();
    let parameters = test_lame_parameters().collect::<Vec<_>>();

    let particle_parameters_host: Vec<_> = parameters
        .iter()
        .cloned()
        .flat_map(|p| repeat(p).take(position_gradients_host.len()))
        .collect();
    let position_gradients_host = position_gradients_host.repeat(parameters.len());
    assert_eq!(
        particle_parameters_host.len(),
        position_gradients_host.len()
    );

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
    let particle_parameters_device: Vec<_> = particle_parameters_host
        .iter()
        .cloned()
        .map(Into::into)
        .collect();

    let (stresses_cpu, energies_cpu): (Vec<Matrix3<f32>>, Vec<f32>) = position_gradients_host
        .iter()
        .zip(particle_parameters_host.clone())
        .map(|(position_gradient, parameters)| match parameters {
            Host::Solid(Solid { mu, lambda, .. }) => (
                first_piola_stress_neo_hookean(mu, lambda, position_gradient),
                try_elastic_energy_neo_hookean(mu, lambda, position_gradient).unwrap(),
            ),
            Host::Fluid(Fluid {
                exponent,
                bulk_modulus,
                ..
            }) => (
                first_piola_stress_inviscid(bulk_modulus, exponent, position_gradient),
                elastic_energy_inviscid(bulk_modulus, exponent, position_gradient),
            ),
        })
        .unzip();

    let (stresses_gpu, energies_gpu) = run_elastic(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
        },
        (u16::MAX as u32).try_into().unwrap(),
        &position_gradients_device,
        &particle_parameters_device,
    );

    for i in 0..position_gradients_host.len() {
        println!("position_gradient: {:?}", position_gradients_host[i]);
        println!(
            "particle_parameters_host: {:?}",
            particle_parameters_host[i]
        );
        println!(
            "particle_parameters_device: {:?}",
            particle_parameters_device[i]
        );
        println!("cpu: {}, {:?}", energies_cpu[i], stresses_cpu[i]);
        println!("gpu: {}, {:?}", energies_gpu[i], stresses_gpu[i]);

        check_iters(
            stresses_cpu[i].iter(),
            stresses_gpu[i].fixed_view::<3, 3>(0, 0).iter(),
        );
        assert_relative_eq!(energies_cpu[i], energies_gpu[i], max_relative = 0.01);
    }
}

fn run_elastic(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    position_gradients: &[Matrix4x3<f32>],
    particle_parameters: &[particle_parameters::Device],
) -> (Vec<Matrix4x3<f32>>, Vec<f32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        position_gradients,
        particle_parameters,
    );

    let elastic = Elastic::new(&context, settings);
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
