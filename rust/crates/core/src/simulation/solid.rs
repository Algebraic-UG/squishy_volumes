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

use crate::{
    elastic::{lambda_stable_neo_hookean, mu_stable_neo_hookean, try_elastic_energy_neo_hookean},
    error_messages::SAMPLING_FAILED,
    report::Report,
    setup::{GlobalSettings, Mesh, ObjectSettingsSolid},
    simulation::{particles::ParticleParameters, state::profile},
};
use anyhow::{Result, ensure};
use nalgebra::{Matrix3, Vector3};
use serde::{Deserialize, Serialize};

use tracing::info;

use super::{kinematic::Kinematic, particles::Particles};

#[derive(Clone, Serialize, Deserialize)]
pub struct Solid {
    pub particles: Vec<usize>,
}

pub struct SolidConstruction<'a> {
    pub name: &'a str,
    pub run: Arc<AtomicBool>,
    pub report: Report,
    pub settings: &'a GlobalSettings,
    pub kinematic: Kinematic,
    pub object_settings: ObjectSettingsSolid,
    pub mesh: &'a Mesh,
    pub particles: &'a mut Particles,
}

impl Solid {
    pub fn new(
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
            particles,
        }: SolidConstruction,
    ) -> Result<Self> {
        info!("solid object");

        let report = report.new_sub(crate::ReportInfo {
            name: format!("Creating Solid '{name}'"),
            completed_steps: 0,
            steps_to_completion: NonZero::new(2).unwrap(),
        });

        let mu = mu_stable_neo_hookean(youngs_modulus, poissons_ratio);
        let lambda = lambda_stable_neo_hookean(youngs_modulus, poissons_ratio);
        info!(mu, lambda, "Lamé parameters");
        let particle_volume = particle_size.powi(3);

        let position_gradient = Matrix3::from(orientation.to_rotation_matrix()) * dilation;

        let velocity_gradient = Matrix3::from_columns(&[
            Vector3::new(0., angular_velocity.z, -angular_velocity.y),
            Vector3::new(-angular_velocity.z, 0., angular_velocity.x),
            Vector3::new(angular_velocity.y, -angular_velocity.x, 0.),
        ]);

        ensure!(dilation > 0., "dilation must be positive");

        let mass = particle_volume * density;
        let elastic_energy = try_elastic_energy_neo_hookean(mu, lambda, &position_gradient)?;
        let samples = mesh.sample_inside(
            run.clone(),
            report.clone(),
            *particle_size * dilation,
            randomness,
        )?;
        let first_idx = particles.sort_map.len();
        report.step();
        {
            profile!("fill vectors");
            let Particles {
                sort_map,
                reverse_sort_map,
                states,
                parameters,
                masses,
                initial_volumes,
                initial_positions,
                positions,
                position_gradients,
                velocities,
                velocity_gradients,
                elastic_energies,
                collider_insides,
                trial_position_gradients: _,
                action_matrices: _,
            } = particles;
            ensure!(!samples.is_empty(), SAMPLING_FAILED);

            let n = first_idx + samples.len();
            sort_map.extend(first_idx..n);
            reverse_sort_map.extend(first_idx..n);

            states.resize(n, Default::default());
            parameters.resize(
                n,
                ParticleParameters::Solid {
                    mu,
                    lambda,
                    viscosity,
                    sand_alpha,
                },
            );
            masses.resize(n, mass);
            initial_volumes.resize(n, particle_volume);
            position_gradients.resize(n, position_gradient);
            velocity_gradients.resize(n, velocity_gradient);
            elastic_energies.resize(n, elastic_energy);
            collider_insides.resize(n, Default::default());

            initial_positions.extend(
                samples
                    .iter()
                    .map(|sample| orientation.transform_vector(sample) + position),
            );
            positions.extend(
                samples
                    .iter()
                    .map(|sample| orientation.transform_vector(sample) + position),
            );
            velocities.extend(samples.iter().map(|sample| {
                linear_velocity + angular_velocity.cross(&orientation.transform_vector(sample))
            }));
            info!(number_of_particles = samples.len(), "new solid object");
        }
        report.step();

        Ok(Self {
            particles: (first_idx..first_idx + samples.len()).collect(),
        })
    }
}
