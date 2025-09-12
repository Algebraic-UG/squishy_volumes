// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{num::NonZero, sync::Arc};

use anyhow::{Context, Result, ensure};
use serde_json::{Value, from_value, to_value};
use squishy_volumes_api::{T, Task};
use tracing::warn;

use crate::{PhaseInput, Simulation, api::ObjectWithData, math::flat::Flat3};

use super::{
    cache::Cache,
    compute_thread::ComputeThread,
    state::attributes::{Attribute, AttributeMesh, AttributeSetting},
};

pub struct SimulationLocal {
    cache: Arc<Cache>,
    compute_thread: Option<ComputeThread>,
}

impl SimulationLocal {
    pub fn new(cache: Cache) -> Self {
        Self {
            cache: cache.into(),
            compute_thread: None,
        }
    }
}

impl Simulation for SimulationLocal {
    fn computing(&self) -> bool {
        self.compute_thread
            .as_ref()
            .is_some_and(ComputeThread::running)
    }

    fn poll(&mut self) -> Result<Option<Task>> {
        self.cache.check()?;
        self.compute_thread
            .as_mut()
            .map(ComputeThread::poll)
            .unwrap_or(Ok(Default::default()))
    }

    fn start_compute(
        &mut self,
        time_step: T,
        explicit: bool,
        debug_mode: bool,
        next_frame: usize,
        number_of_frames: usize,
        max_bytes_on_disk: u64,
    ) -> Result<()> {
        self.cache.set_max_bytes_on_disk(max_bytes_on_disk);

        let Some(number_of_frames) = NonZero::new(number_of_frames) else {
            warn!("asked to compute 0 frames");
            return Ok(());
        };

        ensure!(next_frame < number_of_frames.get(), "no point in computing");
        ensure!(
            next_frame <= self.available_frames(),
            "frame not computed yet"
        );

        self.pause_compute();
        self.cache.check()?;
        self.cache.drop_frames(next_frame)?;
        self.compute_thread = Some(ComputeThread::new(
            self.cache.clone(),
            self.cache.setup.settings.frames_per_second as usize,
            PhaseInput {
                max_time_step: time_step,
                time_step,
                explicit,
                debug_mode,
                setup: self.cache.setup.clone(),
            },
            number_of_frames,
            next_frame,
        )?);
        Ok(())
    }

    fn pause_compute(&mut self) {
        self.compute_thread = None
    }

    fn available_frames(&self) -> usize {
        self.cache.available_frames()
    }

    fn available_attributes(&self, frame: usize) -> Result<Vec<Value>> {
        self.cache
            .available_attributes(frame)?
            .into_iter()
            .map(|attribute| Ok(to_value(attribute)?))
            .collect()
    }

    fn fetch_flat_attribute(&self, frame: usize, attribute: Value) -> Result<Vec<T>> {
        let attribute = from_value(attribute)?;
        match attribute {
            Attribute::Setting(attribute) => {
                let settings = &self.cache.setup.settings;
                Ok(match attribute {
                    AttributeSetting::GridNodeSize => vec![settings.grid_node_size],
                    AttributeSetting::ParticleSize => vec![settings.particle_size],
                    AttributeSetting::FramesPerSecond => {
                        vec![settings.frames_per_second as T]
                    }
                    AttributeSetting::Gravity => settings.gravity.flat().into(),
                })
            }
            Attribute::Mesh { name, attribute } => {
                let ObjectWithData { object, mesh, .. } = self
                    .cache
                    .setup
                    .objects
                    .iter()
                    .find(|object_with_data| object_with_data.object.name == name)
                    .context("Missing object")?;
                Ok(match attribute {
                    AttributeMesh::Vertices => mesh
                        .vertices
                        .iter()
                        .flat_map(|v| v.iter().cloned())
                        .collect(),
                    AttributeMesh::Triangles => mesh
                        .triangles
                        .iter()
                        .flat_map(|indices| indices.iter().map(|i| *i as T))
                        .collect(),
                    AttributeMesh::Scale => object.scale.iter().cloned().collect(),
                    AttributeMesh::Position => object.position.iter().cloned().collect(),
                    AttributeMesh::Orientation => {
                        object.orientation.coords.iter().cloned().collect()
                    }
                })
            }
            attribute => self.cache.fetch_flat_attribute(frame, attribute),
        }
    }
}
