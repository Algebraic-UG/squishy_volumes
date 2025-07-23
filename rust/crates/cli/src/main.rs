// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{ensure, Result};
use blended_mpm_core::{Cache, Phase, PhaseInput, Report, ReportInfo, State};
use std::{
    num::NonZero,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tracing::{info, subscriber::set_global_default};
use tracing_subscriber::FmtSubscriber;
use uuid::Uuid;

use clap::Parser;

#[derive(Parser)]
struct Cli {
    #[arg(long, value_name = "PATH_TO_CACHE_DIR")]
    cache_dir: PathBuf,

    #[arg(long, value_name = "TIME_STEP")]
    time_step: f32,

    #[arg(long)]
    explicit: bool,

    #[arg(long)]
    debug_mode: bool,

    #[arg(long, value_name = "CHECK_POINT")]
    start_frame: Option<usize>,

    #[arg(long, value_name = "NUMBER_OF_STEPS")]
    number_of_sub_frames: Option<usize>,

    #[arg(long, value_name = "NUMBER_OF_FRAMES")]
    number_of_frames: Option<usize>,

    #[arg(long)]
    no_output: bool,
}

fn main() -> Result<()> {
    set_global_default(FmtSubscriber::default())?;
    let Cli {
        cache_dir,
        time_step,
        explicit,
        debug_mode,
        start_frame,
        number_of_sub_frames,
        number_of_frames,
        no_output,
    } = Cli::parse();

    let output_profile = || -> Result<()> {
        #[cfg(feature = "profile")]
        {
            let mut buf = std::io::BufWriter::new(Vec::new());
            coarse_prof::write(&mut buf)?;
            info!("{}", String::from_utf8(buf.into_inner()?)?);
            coarse_prof::reset();
        }
        Ok(())
    };

    let run = Arc::new(AtomicBool::new(true));
    {
        let run = run.clone();
        ctrlc::set_handler(move || {
            run.store(false, Ordering::Relaxed);
        })?;
    }

    let cache = Cache::load(Uuid::new_v4().to_string(), cache_dir, 10_000_000_000)?;
    let seconds_per_frame = 1. / cache.setup.settings.frames_per_second as f64;

    ensure!(
        start_frame.is_none_or(|start_frame| start_frame < cache.available_frames()),
        "requested check point not found"
    );

    let mut completed_sub_frames = 0;
    let mut completed_frames = 0;

    let mut next_frame = start_frame
        .map(|f| f + 1)
        .unwrap_or(cache.available_frames());
    if !no_output {
        cache.drop_frames(next_frame)?;
    }

    let stamp = Instant::now();

    let mut current_state = if next_frame == 0 {
        let state = State::new(
            Arc::new(AtomicBool::new(true)),
            Report::new(ReportInfo {
                name: "".to_string(),
                completed_steps: 0,
                steps_to_completion: NonZero::new(1).unwrap(),
            }),
            &cache.setup,
        )?;
        if !no_output {
            cache.store_frame(state.clone())?;
        }
        output_profile()?;
        next_frame += 1;
        completed_frames += 1;
        state
    } else {
        cache.fetch_frame(next_frame - 1)?
    };

    while run.load(Ordering::Relaxed)
        && number_of_sub_frames.is_none_or(|n| n > completed_sub_frames)
        && number_of_frames.is_none_or(|n| n > completed_frames)
    {
        current_state = current_state.next(&mut PhaseInput {
            time_step,
            max_time_step: time_step,
            explicit,
            debug_mode,
            setup: cache.setup.clone(),
        })?;
        if current_state.phase() != Phase::default() {
            continue;
        }

        completed_sub_frames += 1;

        info!(
                "simulated_time: {:0.4}, real_time: {:0.4}, ratio: {:0.4}, per_subframe: {:0.4}, per_frame: {:0.4}",
                current_state.time(),
                stamp.elapsed().as_secs_f64(),
                stamp.elapsed().as_secs_f64() / current_state.time(),
                stamp.elapsed().as_secs_f64() / completed_sub_frames as f64,
                stamp.elapsed().as_secs_f64() / completed_frames as f64,
            );

        let simulated_time = current_state.time();
        let next_stored_frame_time = next_frame as f64 * seconds_per_frame;
        if simulated_time <= next_stored_frame_time {
            continue;
        }

        if !no_output {
            cache.store_frame(current_state.clone())?;
        }
        output_profile()?;
        next_frame += 1;
        completed_frames += 1;
    }

    Ok(())
}
