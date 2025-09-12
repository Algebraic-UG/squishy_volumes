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
    api::{GlobalSettings, Mesh, ObjectSettingsCollider, ScriptedFrame, SurfaceSample},
    math::NORMALIZATION_EPS,
    report::Report,
};
use anyhow::{Context, Result};
use nalgebra::{UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use tracing::info;

use super::kinematic::{Kinematic, ScriptedMovement};

#[derive(Clone, Serialize, Deserialize)]
pub struct Collider {
    pub sticky_factor: T,
    pub friction_factor: T,

    pub surface_samples: Vec<SurfaceSample>,

    pub kinematic: Kinematic,
    pub has_moved: bool,

    // TODO: this doesn't need to be stored in each state
    pub scripted_movements: Vec<ScriptedMovement>,
}

pub struct ColliderConstruction<'a> {
    pub name: &'a str,
    pub run: Arc<AtomicBool>,
    pub report: Report,
    pub settings: &'a GlobalSettings,
    pub kinematic: Kinematic,
    pub object_settings: ObjectSettingsCollider,
    pub mesh: &'a Mesh,
    pub scripted_frames: Vec<ScriptedFrame>,
}

impl Collider {
    pub fn new(
        ColliderConstruction {
            name,
            run,
            report,
            settings:
                GlobalSettings {
                    grid_node_size,
                    frames_per_second,
                    ..
                },
            kinematic,
            object_settings:
                ObjectSettingsCollider {
                    sticky_factor,
                    friction_factor,
                },
            mesh,
            scripted_frames,
        }: ColliderConstruction,
    ) -> Result<Self> {
        info!("collider object");

        let report = report.new_sub(crate::ReportInfo {
            name: format!("Creating Collider '{name}'"),
            completed_steps: 0,
            steps_to_completion: NonZero::new(2).unwrap(),
        });

        let seconds_per_frame = 1. / (*frames_per_second as T);
        let scripted_movements = scripted_frames
            .into_iter()
            .enumerate()
            .map(
                |(
                    frame,
                    ScriptedFrame {
                        position,
                        orientation,
                    },
                )| {
                    Ok(ScriptedMovement {
                        time: seconds_per_frame * frame as T,
                        position,
                        orientation: UnitQuaternion::try_new(orientation, NORMALIZATION_EPS)
                            .context("Orientation not normalized")?,
                    })
                },
            )
            .collect::<Result<Vec<_>>>()
            .context("Scripted frames parsing")?;
        report.step();

        let surface_samples = mesh.sample_surface(run, *grid_node_size / 2.)?;
        report.step();

        Ok(Self {
            sticky_factor,
            friction_factor,
            surface_samples,
            kinematic,
            has_moved: true,
            scripted_movements,
        })
    }

    pub fn conform_velocity(
        &self,
        position: Vector3<T>,
        velocity: Vector3<T>,
        normal: Vector3<T>,
        approach: T,
    ) -> Vector3<T> {
        let point_velocity = self.kinematic.point_velocity_from_world(position);

        let relative_velocity = velocity - point_velocity;

        let normal_part = normal.dot(&relative_velocity);
        let normal_velocity = normal * normal_part;
        let tangent_velocity = relative_velocity - normal_velocity;
        let tangent_part = tangent_velocity.norm();

        point_velocity
            + tangent_velocity
                * if normal_part < 0. && tangent_part > 0. {
                    (1. + self.friction_factor * normal_part / tangent_part).max(0.)
                } else {
                    1.
                }
            + normal_velocity
                * if normal_part > 0. {
                    1. - self.sticky_factor
                } else {
                    approach
                }
    }
}
