// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::Result;
use squishy_volumes_api::{ComputeSettings, Simulation as _};
use squishy_volumes_core::SimulationImpl;
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
use uuid::Uuid;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, value_name = "SIMULATION_DIRECTORY")]
    directory: PathBuf,

    #[arg(long, value_name = "TIME_STEP")]
    time_step: f32,

    #[arg(long)]
    explicit: bool,

    #[arg(long)]
    debug_mode: bool,

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
        explicit,
        debug_mode,
        adaptive_time_steps,
        next_frame,
        number_of_frames,
        max_bytes_on_disk,
    } = Cli::parse();

    let mut simulation = SimulationImpl::load(Uuid::new_v4().to_string(), directory)?;

    let next_frame = next_frame.unwrap_or(simulation.available_frames());

    simulation.start_compute(ComputeSettings {
        time_step,
        explicit,
        debug_mode,
        adaptive_time_steps,
        next_frame,
        number_of_frames,
        max_bytes_on_disk,
    })?;

    let run = Arc::new(AtomicBool::new(true));
    ctrlc::set_handler({
        let run = run.clone();
        move || {
            run.store(false, Ordering::Relaxed);
        }
    })?;

    while run.load(Ordering::Relaxed) && simulation.poll()?.is_some() {
        sleep(Duration::from_millis(200));
    }

    Ok(())
}
