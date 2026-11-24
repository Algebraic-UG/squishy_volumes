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
        if let Some(panic_message) = self
            .simulation_input
            .as_ref()
            .and_then(|simulation_input| simulation_input.panic_message.get())
        {
            return Err(Error::BrokenSimulationInput(panic_message.clone()));
        }
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
        if self
            .simulations
            .get(&uuid)
            .is_some_and(SimulationImpl::is_broken)
        {
            info!("Dropping broken simulation before reloading it");
            self.simulations.remove(&uuid);
        }
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

#[cfg(test)]
mod tests {
    use serde_json::json;
    use squishy_volumes_api::Context;
    use tempfile::TempDir;

    use super::*;
    use crate::{
        compute_thread::ComputeThread,
        guard::{Breakable, guard_breakable},
    };

    const PANIC_MESSAGE: &str = "api test panic";

    fn input_header() -> Value {
        json!({
            "consts": {
                "grid_node_size": 0.5,
                "leaf_size": 1.0,
                "leaf_threshold": 16,
                "simulation_scale": 1.0,
                "frames_per_second": 24,
                "domain_min": [-1.0, -1.0, -1.0],
                "domain_max": [1.0, 1.0, 1.0],
                "capture_start_frame": 1
            },
            "objects": {}
        })
    }

    fn context_with_input(directory: &TempDir) -> ContextImpl {
        let mut context = ContextImpl::default();
        context
            .new_simulation_input(
                "test".to_string(),
                directory.path().to_path_buf(),
                input_header(),
                u64::MAX,
            )
            .unwrap();
        context
    }

    fn panic_inside<B: Breakable>(breakable: &mut B) {
        let result = guard_breakable(breakable, |_| -> Result<(), Error> {
            panic!("{PANIC_MESSAGE}")
        });
        assert!(matches!(
            result,
            Err(Error::Panic { thread: "caller", message }) if message == PANIC_MESSAGE
        ));
    }

    fn core_error(error: anyhow::Error) -> Error {
        error.downcast().unwrap()
    }

    #[test]
    fn panicking_context_method_is_a_caller_panic() {
        let result = crate::guard::guard(|| -> Result<(), Error> { panic!("{PANIC_MESSAGE}") });

        assert!(matches!(
            result,
            Err(Error::Panic { thread: "caller", message }) if message == PANIC_MESSAGE
        ));
    }

    #[test]
    fn panicking_simulation_method_leaves_it_broken() {
        let directory = TempDir::new().unwrap();
        let mut context = context_with_input(&directory);
        let uuid = context.new_simulation().unwrap();

        panic_inside(context.simulations.get_mut(&uuid).unwrap());

        let simulation = context.get_simulation_mut(&uuid).unwrap();
        let errors = [
            simulation.input_header().unwrap_err(),
            simulation.baking().unwrap_err(),
            simulation.poll().unwrap_err(),
            simulation.available_frames().unwrap_err(),
        ];
        for error in errors {
            assert!(matches!(
                core_error(error),
                Error::BrokenSimulation(message) if message == PANIC_MESSAGE
            ));
        }
    }

    #[test]
    fn broken_simulation_can_still_stop_its_computation() {
        let directory = TempDir::new().unwrap();
        let mut context = context_with_input(&directory);
        let uuid = context.new_simulation().unwrap();
        let simulation = context.simulations.get_mut(&uuid).unwrap();
        simulation.compute_thread = Some(ComputeThread::spawn_work(ComputeThread::until_canceled));
        panic_inside(simulation);

        let simulation = context.get_simulation_mut(&uuid).unwrap();
        simulation.pause_compute().unwrap();

        assert!(!context.simulations[&uuid].baking_impl());
    }

    #[test]
    fn broken_simulation_can_be_reloaded() {
        let directory = TempDir::new().unwrap();
        let mut context = context_with_input(&directory);
        let uuid = context.new_simulation().unwrap();
        panic_inside(context.simulations.get_mut(&uuid).unwrap());

        context
            .load_simulation(uuid.clone(), directory.path().to_path_buf())
            .unwrap();

        let simulation = context.get_simulation_mut(&uuid).unwrap();
        assert!(simulation.poll().is_ok());
        assert_eq!(simulation.available_frames().unwrap(), 0);
    }

    #[test]
    fn panicking_input_method_leaves_it_broken() {
        let directory = TempDir::new().unwrap();
        let mut context = context_with_input(&directory);

        panic_inside(context.simulation_input.as_mut().unwrap());

        let simulation_input = context.get_simulation_input().unwrap();
        assert!(matches!(
            core_error(simulation_input.start_frame(json!({})).unwrap_err()),
            Error::BrokenSimulationInput(message) if message == PANIC_MESSAGE
        ));
        assert!(matches!(
            core_error(context.new_simulation().unwrap_err()),
            Error::BrokenSimulationInput(message) if message == PANIC_MESSAGE
        ));
        assert!(context.simulation_input.is_some());
        assert!(context.drop_simulation_input().is_ok());
    }
}
