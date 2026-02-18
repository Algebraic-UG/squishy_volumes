// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, path::PathBuf};

use anyhow::{Context as _, Result, bail};
use itertools::multiunzip;
use nalgebra::Vector3;
use serde_json::{Value, from_value};
use squishy_volumes_api::{Context, Simulation, SimulationInput, T};
use tracing::{info, subscriber::set_global_default, warn};
use tracing_subscriber::FmtSubscriber;

use crate::{math::flat::Flat3, rasterization::rasterize, state::grids::WeightedDistance};

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
        if self.simulation_input.take().is_none() {
            warn!("No simulation input")
        }
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

    fn test(&mut self, data: &[f32]) -> Vec<f32> {
        let spacing = data[0];
        let layers = data[1] as usize;
        let mut chunks = data[2..].chunks_exact(3);
        let corner_a = Vector3::from_column_slice(chunks.next().unwrap());
        let corner_b = Vector3::from_column_slice(chunks.next().unwrap());
        let corner_c = Vector3::from_column_slice(chunks.next().unwrap());
        let opposite_d = Vector3::from_column_slice(chunks.next().unwrap());
        let opposite_e = Vector3::from_column_slice(chunks.next().unwrap());
        let opposite_f = Vector3::from_column_slice(chunks.next().unwrap());

        let (positions, distances, normals): (Vec<_>, Vec<_>, Vec<_>) =
            multiunzip(
                rasterize(
                    [&corner_a, &corner_b, &corner_c],
                    [Some(&opposite_d), Some(&opposite_e), Some(&opposite_f)],
                    spacing,
                    layers,
                )
                .map(
                    |(
                        grid_node,
                        WeightedDistance {
                            distance,
                            normal,
                        },
                    )|
                     -> (Vector3<T>,  T, Vector3<T>) {
                        (
                            grid_node.map(|c| c as T * spacing),
                            distance,
                            normal,
                        )
                    },
                ),
            );

        positions
            .into_iter()
            .flat_map(|v| v.flat())
            .chain(normals.into_iter().flat_map(|v| v.flat()))
            .chain(distances)
            .collect()
    }
}
