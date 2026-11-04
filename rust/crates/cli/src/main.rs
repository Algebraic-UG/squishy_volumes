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

use anyhow::Result;
use squishy_volumes_core::{ComputeSettings, SimulationImpl};
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::sleep,
    time::Duration,
};
use tracing::subscriber::set_global_default;
use tracing_subscriber::FmtSubscriber;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, value_name = "SIMULATION_DIRECTORY")]
    directory: PathBuf,

    #[arg(long, value_name = "TIME_STEP")]
    time_step: f32,

    #[arg(long)]
    gpu: bool,

    #[arg(long)]
    adaptive_time_steps: bool,

    #[arg(long, value_name = "NUMBER_OF_FRAMES")]
    number_of_frames: usize,

    #[arg(long, value_name = "CHECK_POINT")]
    next_frame: Option<usize>,

    #[arg(long, value_name = "NUMBER_OF_BYTES")]
    max_bytes_on_disk: u64,
}

fn main() -> Result<()> {
    set_global_default(FmtSubscriber::default())?;
    let Cli {
        directory,
        time_step,
        gpu,
        adaptive_time_steps,
        next_frame,
        number_of_frames,
        max_bytes_on_disk,
    } = Cli::parse();

    let mut simulation = SimulationImpl::load(directory)?;

    let next_frame = next_frame.unwrap_or(simulation.available_frames_impl());

    let gpu = gpu.then(|| {
        squishy_volumes_gpu::GpuContext::available_gpus()
            .first()
            .cloned()
            .expect("No gpus found")
    });

    simulation.start_compute_impl(
        serde_json::to_value(ComputeSettings {
            time_step,
            gpu,
            adaptive_time_steps,
            next_frame,
            number_of_frames,
            max_bytes_on_disk,
        })
        .unwrap(),
    )?;

    let run = Arc::new(AtomicBool::new(true));
    ctrlc::set_handler({
        let run = run.clone();
        move || {
            run.store(false, Ordering::Relaxed);
        }
    })?;

    while run.load(Ordering::Relaxed) && simulation.computing_impl() {
        sleep(Duration::from_millis(200));
    }

    simulation.poll_impl()?;

    Ok(())
}
