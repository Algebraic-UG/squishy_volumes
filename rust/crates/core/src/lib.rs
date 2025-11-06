// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Context as _;
use anyhow::Result;
use std::path::PathBuf;
use std::{collections::BTreeMap, time::Instant};

use simulation::SimulationLocal;
use squishy_volumes_api::{Context, Simulation};
use tracing::{error, info, subscriber::set_global_default};
use tracing_subscriber::FmtSubscriber;

mod math;
mod report;
mod setup;
mod simulation;

pub use report::{Report, ReportInfo};
pub use simulation::{Phase, PhaseInput, State, cache::Cache, weights};

pub struct ContextImpl(BTreeMap<String, SimulationLocal>);

impl Default for ContextImpl {
    fn default() -> Self {
        if let Err(e) = set_global_default(FmtSubscriber::default()) {
            eprintln!("{e:?}");
        } else {
            info!("initialized");
        }
        Self(Default::default())
    }
}

impl Context for ContextImpl {
    fn new_simulation(
        &mut self,
        uuid: String,
        cache_dir: PathBuf,
        setup: serde_json::Value,
        max_bytes_on_disk: u64,
    ) -> Result<()> {
        let stamp = Instant::now();
        self.0.remove(&uuid);

        self.0.insert(
            uuid.clone(),
            SimulationLocal::new(
                Cache::new(uuid, setup, cache_dir.clone(), max_bytes_on_disk)
                    .with_context(|| format!("failed to prepare cache: {cache_dir:?}"))?,
            ),
        );
        info!(
            took = stamp.elapsed().as_secs_f32(),
            "New simulation is ready."
        );
        Ok(())
    }

    fn load_simulation(
        &mut self,
        uuid: String,
        cache_dir: PathBuf,
        max_bytes_on_disk: u64,
    ) -> anyhow::Result<()> {
        let stamp = Instant::now();
        self.0.remove(&uuid);

        self.0.insert(
            uuid.clone(),
            SimulationLocal::new(
                Cache::load(uuid, cache_dir.clone(), max_bytes_on_disk)
                    .with_context(|| format!("failed to load cache: {cache_dir:?}"))?,
            ),
        );
        info!(
            took = stamp.elapsed().as_secs_f32(),
            "New simulation is ready."
        );
        Ok(())
    }

    fn get_simulation(&self, uuid: &str) -> Result<&dyn Simulation> {
        self.0
            .get(uuid)
            .map(|r| r as &dyn Simulation)
            .with_context(|| format!("no simulation with {uuid}"))
    }

    fn get_simulation_mut(&mut self, uuid: &str) -> Result<&mut dyn Simulation> {
        self.0
            .get_mut(uuid)
            .map(|r| r as &mut dyn Simulation)
            .with_context(|| format!("no simulation with {uuid}"))
    }

    fn drop_simulation(&mut self, uuid: &str) -> Result<()> {
        if self.0.remove(uuid).is_none() {
            error!("asked to remove non-existent simulation with {uuid}")
        }
        Ok(())
    }
}
