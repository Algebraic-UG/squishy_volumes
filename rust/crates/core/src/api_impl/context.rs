// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use serde_json::{Value, from_value};
use squishy_volumes_api::{Context, Simulation, SimulationInput};
use tracing::{info, subscriber::set_global_default, warn};
use tracing_subscriber::FmtSubscriber;

use super::{SimulationImpl, SimulationInputImpl};

pub struct ContextImpl {
    simulation_input: Option<SimulationInputImpl>,
    simulations: BTreeMap<String, SimulationImpl>,
}

impl Default for ContextImpl {
    fn default() -> Self {
        if let Err(e) = set_global_default(FmtSubscriber::default()) {
            eprintln!("{e:?}");
        } else {
            info!("initialized");
        }
        Self {
            simulation_input: Default::default(),
            simulations: Default::default(),
        }
    }
}

impl Context for ContextImpl {
    fn new_simulation_input(
        &mut self,
        uuid: String,
        directory: PathBuf,
        input_header: Value,
        max_bytes_on_disk: u64,
    ) -> Result<()> {
        let input_header = from_value(input_header).context("Parsing input header.")?;

        if self.simulation_input.is_some() {
            warn!("Overwriting old input.");
        }

        self.simulation_input = Some(SimulationInputImpl::new(
            uuid,
            directory,
            input_header,
            max_bytes_on_disk,
        )?);

        Ok(())
    }

    fn get_simulation_input(&mut self) -> Option<&mut dyn SimulationInput> {
        self.simulation_input
            .as_mut()
            .map(|r| r as &mut dyn SimulationInput)
    }

    fn drop_simulation_input(&mut self) {
        let Some(simulation_input) = self.simulation_input.take() else {
            warn!("No simulation input");
            return;
        };
        simulation_input.clean_up();
    }

    fn new_simulation(&mut self) -> Result<String> {
        let Some(simulation_input) = self.simulation_input.take() else {
            bail!("No input prepared.");
        };

        let uuid = simulation_input.directory_lock.uuid().to_string();
        let simulation = SimulationImpl::new(simulation_input)?;

        if self.simulations.insert(uuid.clone(), simulation).is_some() {
            warn!("Overwriting old simulation");
        }

        Ok(uuid)
    }

    fn load_simulation(&mut self, uuid: String, directory: PathBuf) -> Result<()> {
        let simulation = SimulationImpl::load(uuid.clone(), directory)?;

        if self.simulations.insert(uuid, simulation).is_some() {
            warn!("Overwriting old simulation");
        }

        Ok(())
    }

    fn get_simulation(&self, uuid: &str) -> Option<&dyn Simulation> {
        self.simulations.get(uuid).map(|r| r as &dyn Simulation)
    }

    fn get_simulation_mut(&mut self, uuid: &str) -> Option<&mut dyn Simulation> {
        self.simulations
            .get_mut(uuid)
            .map(|r| r as &mut dyn Simulation)
    }

    fn drop_simulation(&mut self, uuid: &str) {
        if self.simulations.remove(uuid).is_none() {
            warn!("No simulation with {uuid}")
        }
    }
}
