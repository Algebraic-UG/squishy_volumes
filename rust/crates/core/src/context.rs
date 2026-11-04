// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use std::{collections::BTreeMap, path::PathBuf};

use serde_json::{Value, from_value};
use squishy_volumes_api::{Simulation, SimulationInput};
use tracing::{info, subscriber::set_global_default, warn};
use tracing_subscriber::FmtSubscriber;

use super::{Error, SimulationImpl, SimulationInputImpl};

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

#[allow(clippy::result_large_err)]
impl ContextImpl {
    pub fn new_simulation_input_impl(
        &mut self,
        uuid: String,
        directory: PathBuf,
        input_header: Value,
        max_bytes_on_disk: u64,
    ) -> Result<(), Error> {
        let input_header = from_value(input_header).map_err(Error::ParsingInputHeader)?;

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

    pub fn get_simulation_input_impl(&mut self) -> Option<&mut dyn SimulationInput> {
        self.simulation_input
            .as_mut()
            .map(|r| r as &mut dyn SimulationInput)
    }

    pub fn drop_simulation_input_impl(&mut self) {
        let Some(simulation_input) = self.simulation_input.take() else {
            warn!("No simulation input");
            return;
        };
        simulation_input.clean_up();
    }

    pub fn new_simulation_impl(&mut self) -> Result<String, Error> {
        let Some(simulation_input) = self.simulation_input.take() else {
            return Err(Error::MissingInput)?;
        };

        let uuid = simulation_input.uuid.clone();
        let simulation = SimulationImpl::new(simulation_input)?;

        if self.simulations.insert(uuid.clone(), simulation).is_some() {
            warn!("Overwriting old simulation");
        }

        Ok(uuid)
    }

    pub fn load_simulation_impl(&mut self, uuid: String, directory: PathBuf) -> Result<(), Error> {
        let simulation = SimulationImpl::load(directory)?;

        if self.simulations.insert(uuid, simulation).is_some() {
            warn!("Overwriting old simulation");
        }

        Ok(())
    }

    pub fn get_simulation_impl(&self, uuid: &str) -> Option<&dyn Simulation> {
        self.simulations.get(uuid).map(|r| r as &dyn Simulation)
    }

    pub fn get_simulation_mut_impl(&mut self, uuid: &str) -> Option<&mut dyn Simulation> {
        self.simulations
            .get_mut(uuid)
            .map(|r| r as &mut dyn Simulation)
    }

    pub fn drop_simulation_impl(&mut self, uuid: &str) {
        if self.simulations.remove(uuid).is_none() {
            warn!("No simulation with {uuid}")
        }
    }
}
