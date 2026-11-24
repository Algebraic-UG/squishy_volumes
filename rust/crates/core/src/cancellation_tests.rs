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

use std::num::NonZero;

use squishy_volumes_cpu::{CpuRunParameters, CpuState};
use squishy_volumes_file_input::{
    InputHeader, InputObject, InputObjectParticles, InputReader, write_test_input,
};
use squishy_volumes_gpu::{GpuContext, GpuRunParameters, GpuState};
use squishy_volumes_xpu::Harness;
use tempfile::TempDir;

use crate::initialization::{InitializationError, initialize_io_state};

fn recorded_input(directory: &TempDir) -> InputReader {
    let particles = InputObject::Particles(InputObjectParticles {
        num_particles: 1,
        alembic_attributes: Default::default(),
    });
    write_test_input(
        directory.path().join("input.bin"),
        InputHeader::test_input([("particles".to_string(), particles)]),
        2,
    )
}

fn harness() -> Harness {
    Harness::new("test".to_string(), NonZero::new(1).unwrap())
}

fn canceled_harness() -> Harness {
    let harness = harness();
    harness.cancel();
    harness
}

#[test]
fn initialization_with_canceled_harness_is_canceled() {
    let directory = TempDir::new().unwrap();
    let mut input_reader = recorded_input(&directory);

    assert!(matches!(
        initialize_io_state(&canceled_harness(), &mut input_reader),
        Err(InitializationError::Canceled(_))
    ));
}

#[test]
fn cpu_step_with_canceled_harness_is_canceled() {
    let directory = TempDir::new().unwrap();
    let mut input_reader = recorded_input(&directory);
    let io_state = initialize_io_state(&harness(), &mut input_reader).unwrap();
    let mut cpu_state = CpuState::new(0, io_state, input_reader).unwrap();

    let result = cpu_state.produce_next_state(
        &canceled_harness(),
        CpuRunParameters {
            max_time_step: 0.01,
            adaptive_time_steps: true,
            store_bvh: false,
            store_grid: false,
        },
    );

    assert!(matches!(
        result,
        Err(squishy_volumes_cpu::Error::Canceled(_))
    ));
}

#[test]
fn gpu_step_with_canceled_harness_is_canceled() {
    let directory = TempDir::new().unwrap();
    let mut input_reader = recorded_input(&directory);
    let harness = harness();
    let io_state = initialize_io_state(&harness, &mut input_reader).unwrap();
    let gpu = GpuContext::available_gpus().into_iter().next().unwrap();
    let mut gpu_state =
        GpuState::new(0, io_state, input_reader, gpu, &harness, 0.01, None).unwrap();
    harness.cancel();

    let result = gpu_state.produce_next_state(
        &harness,
        GpuRunParameters {
            adaptive_time_steps: true,
            store_bvh: false,
            store_grid: false,
        },
    );

    assert!(matches!(
        result,
        Err(squishy_volumes_gpu::Error::Canceled(_))
    ));
}
