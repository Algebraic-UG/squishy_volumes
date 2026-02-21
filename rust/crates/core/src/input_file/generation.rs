// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    num::NonZero,
    sync::{Arc, atomic::AtomicBool},
};

use nalgebra::Matrix3;
use squishy_volumes_api::T;
use tracing::info;

use crate::{
    Report,
    elastic::{lambda_stable_neo_hookean, mu_stable_neo_hookean, try_elastic_energy_neo_hookean},
    ensure_err,
    input::{BulkData, InputFrame, InputGenerationError},
    math::{
        flat::{Flat3, Flat9},
        velocity_gradient_from_angular_velocity,
    },
    profile,
    setup::{GlobalSettings, Mesh, ObjectSettingsSolid, ViscosityParameters},
    simulation::Kinematic,
};

pub struct SolidConstruction<'a> {
    pub name: &'a str,
    pub run: Arc<AtomicBool>,
    pub report: Report,
    pub settings: &'a GlobalSettings,
    pub kinematic: Kinematic,
    pub object_settings: ObjectSettingsSolid,
    pub mesh: &'a Mesh,
}

pub fn generate_solid_input(
    frame: &mut InputFrame,
    SolidConstruction {
        name,
        run,
        report,
        settings: GlobalSettings { particle_size, .. },
        kinematic:
            Kinematic {
                position,
                orientation,
                linear_velocity,
                angular_velocity,
            },
        object_settings:
            ObjectSettingsSolid {
                density,
                youngs_modulus,
                poissons_ratio,
                viscosity,
                dilation,
                randomness,
                sand_alpha,
            },

        mesh,
    }: SolidConstruction,
) -> Result<(), InputGenerationError> {
    info!("solid object");

    let report = report.new_sub(crate::ReportInfo {
        name: format!("Generating Input for Solid '{name}'"),
        completed_steps: 0,
        steps_to_completion: NonZero::new(2).unwrap(),
    });

    let mu = mu_stable_neo_hookean(youngs_modulus, poissons_ratio);
    let lambda = lambda_stable_neo_hookean(youngs_modulus, poissons_ratio);
    info!(mu, lambda, "Lamé parameters");
    let particle_volume = particle_size.powi(3);

    let position_gradient = Matrix3::from(orientation.to_rotation_matrix()) * dilation;

    let velocity_gradient = velocity_gradient_from_angular_velocity(&angular_velocity);

    ensure_err!(dilation > 0., InputGenerationError::DilationError);

    let mass = particle_volume * density;
    let elastic_energy = try_elastic_energy_neo_hookean(mu, lambda, &position_gradient)?;
    let samples = mesh.sample_inside(
        run.clone(),
        report.clone(),
        *particle_size * dilation,
        randomness,
    )?;
    report.step();
    {
        profile!("fill vectors");
        let n = samples.len();
        ensure_err!(n != 0, InputGenerationError::NoSamples);

        let mut insert_bulk_f32 = |parameter_name: &str, vector: Vec<f32>| {
            ensure_err!(
                frame
                    .bulk
                    .insert(format!("{name}_{parameter_name}"), BulkData::F32(vector))
                    .is_none(),
                InputGenerationError::KeyAlreadyPresent(parameter_name.to_string())
            );
            Ok(())
        };

        let repeat_matrix = |matrix: &Matrix3<T>| -> Vec<f32> {
            matrix
                .flat()
                .into_iter()
                .cycle()
                .take(n * matrix.len())
                .collect()
        };

        insert_bulk_f32("density", vec![density; n])?;
        insert_bulk_f32("youngs_modulus", vec![youngs_modulus; n])?;
        insert_bulk_f32("poissons_ratio", vec![poissons_ratio; n])?;
        if let Some(ViscosityParameters { dynamic, bulk }) = viscosity {
            insert_bulk_f32("viscosity_dynamic", vec![dynamic; n])?;
            insert_bulk_f32("viscosity_bulk", vec![bulk; n])?;
        }
        insert_bulk_f32("dilation", vec![dilation; n])?;
        insert_bulk_f32("randomness", vec![randomness; n])?;
        if let Some(sand_alpha) = sand_alpha {
            insert_bulk_f32("sand_alpha", vec![sand_alpha; n])?;
        }

        insert_bulk_f32("mass", vec![mass; n])?;
        insert_bulk_f32("initial_volume", vec![particle_volume; n])?;
        insert_bulk_f32("position_gradient", repeat_matrix(&position_gradient))?;
        insert_bulk_f32("velocity_gradient", repeat_matrix(&velocity_gradient))?;
        insert_bulk_f32("elastic_energy", vec![elastic_energy; n])?;

        // TODO: collider insides

        let positions: Vec<_> = samples
            .iter()
            .map(|sample| orientation.transform_vector(sample) + position)
            .flat_map(|v| v.flat())
            .collect();
        // TODO: should this be affected by dilation?
        insert_bulk_f32("initial_positions", positions.clone())?;
        insert_bulk_f32("positions", positions)?;
        insert_bulk_f32(
            "velocities",
            samples
                .iter()
                .map(|sample| {
                    linear_velocity + angular_velocity.cross(&orientation.transform_vector(sample))
                })
                .flat_map(|v| v.flat())
                .collect(),
        )?;
        info!(number_of_particles = samples.len(), "new solid object");
    }
    report.step();

    Ok(())
}
