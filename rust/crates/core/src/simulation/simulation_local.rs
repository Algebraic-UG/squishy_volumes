// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, num::NonZero, sync::Arc};

use anyhow::{Context, Result, ensure};
use serde_json::{Value, from_value, to_value};
use squishy_volumes_api::{ComputeSettings, InputBulk, T, Task};
use tracing::{info, warn};

use crate::{
    PhaseInput, Simulation,
    math::flat::Flat3,
    setup::{ComputeStats, ObjectWithData, Stats},
};

use super::{
    cache::Cache,
    compute_thread::ComputeThread,
    state::attributes::{Attribute, AttributeMesh, AttributeSetting},
};

pub struct SimulationLocal {
    cache: Arc<Cache>,
    compute_thread: Option<ComputeThread>,
    cached_compute_stats: Option<ComputeStats>,
}

impl SimulationLocal {
    pub fn new(cache: Cache) -> Self {
        Self {
            cache: cache.into(),
            compute_thread: None,
            cached_compute_stats: None,
        }
    }
}

impl Simulation for SimulationLocal {
    fn record_input(&mut self, meta: Value, bulk: BTreeMap<String, InputBulk>) -> Result<()> {
        info!("recording additional input");
        info!("{meta}");
        info!("bulk keys: {:?}", bulk.keys().collect::<Vec<_>>());
        Ok(())
    }

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
        ComputeSettings {
            time_step,
            explicit,
            debug_mode,
            adaptive_time_steps,
            next_frame,
            number_of_frames,
            max_bytes_on_disk,
        }: ComputeSettings,
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
                time_step_by_velocity: Default::default(),
                time_step_by_deformation: Default::default(),
                time_step_by_isolated: Default::default(),
                time_step_by_sound: Default::default(),
                time_step_by_sound_simple: Default::default(),
                time_step_prior: Default::default(),
                adaptive_time_steps,
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
        self.cached_compute_stats = self
            .compute_thread
            .as_ref()
            .and_then(|compute_thread| compute_thread.stats.lock().unwrap().clone());
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

    fn stats(&self) -> Result<Value> {
        Ok(to_value(Stats {
            loaded_state: self.cache.loaded_state_stats(),
            compute: self
                .compute_thread
                .as_ref()
                .and_then(|compute_thread| compute_thread.stats.lock().unwrap().clone())
                .or(self.cached_compute_stats.clone()),
            bytes_on_disk: self.cache.current_bytes_on_disk(),
        })?)
    }
}
