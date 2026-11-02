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

use std::mem::take;
use std::ops::Range;
use std::path::Path;
use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

use super::GpuContext;

use iter_enumeration::IntoIterEnum2;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ProfilerError {
    #[error("process_finished_frame returned None")]
    NoFinishedFrame,

    #[error("results vector is empty")]
    ResultsEmpty,

    #[error("time is missing")]
    NoTimeRecorded,

    #[error("Layout changed from {old_labels:?} to {labels:?}")]
    LayoutChanged {
        old_labels: Vec<String>,
        labels: Vec<String>,
    },

    #[error("Recorded {steps} steps, but the labels aren't a multiple of that {labels:?}")]
    NotMultipleOfRecordedSteps { steps: usize, labels: Vec<String> },

    #[error("IoError")]
    IoError(#[from] io::Error),
}

fn get_profiling_data(
    context: &GpuContext,
    profiler: &mut wgpu_profiler::GpuProfiler,
) -> Result<Vec<(String, Range<f64>)>, ProfilerError> {
    fn result_to_data(
        result: wgpu_profiler::GpuTimerQueryResult,
    ) -> Box<dyn Iterator<Item = (String, Range<f64>)>> {
        Box::new(
            if let Some(time) = result.time {
                std::iter::once((result.label, time)).iter_enum_2a()
            } else {
                std::iter::empty().iter_enum_2b()
            }
            .chain(result.nested_queries.into_iter().flat_map(result_to_data)),
        )
    }

    let frame = profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .ok_or(ProfilerError::NoFinishedFrame)?;

    Ok(frame.into_iter().flat_map(result_to_data).collect())
}

pub fn profiler_output(
    context: &GpuContext,
    profiler: &mut wgpu_profiler::GpuProfiler,
) -> Result<(), ProfilerError> {
    let mut end = None;
    let mut total_duration = 0.;
    let mut total_inbetween = 0.;
    for (label, time) in get_profiling_data(context, profiler)? {
        let inbetween = if let Some(end) = end {
            (time.start - end) * 1e6
        } else {
            0.
        };
        let duration = (time.end - time.start) * 1e6;
        total_duration += duration;
        total_inbetween += inbetween;
        tracing::info!("{label}: {inbetween:.1}, {duration:.1}");
        end = Some(time.end);
    }
    tracing::info!("{total_inbetween:.1}, {total_duration:.1}");
    println!("XXX: {}", total_duration + total_inbetween);
    Ok(())
}

pub struct ProfileDataCsvWriter {
    steps: usize,
    buffered_data: Vec<(String, Range<f64>)>,
    writer: BufWriter<File>,
    labels: Option<Vec<String>>,
}

impl ProfileDataCsvWriter {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ProfilerError> {
        let writer = BufWriter::new(File::create(path)?);
        Ok(Self {
            steps: 0,
            buffered_data: Default::default(),
            writer,
            labels: None,
        })
    }

    pub fn clear(&mut self) {
        self.steps = 0;
        self.buffered_data.clear();
    }

    pub fn buffer_data(
        &mut self,
        context: &GpuContext,
        profiler: &mut wgpu_profiler::GpuProfiler,
        steps: usize,
    ) -> Result<(), ProfilerError> {
        let mut profiling_data = get_profiling_data(context, profiler)?;
        if profiling_data.is_empty() {
            return Err(ProfilerError::ResultsEmpty);
        }
        if !profiling_data.len().is_multiple_of(steps) {
            return Err(ProfilerError::NotMultipleOfRecordedSteps {
                steps,
                labels: profiling_data
                    .into_iter()
                    .map(|label_and_range| label_and_range.0)
                    .collect(),
            });
        }
        self.buffered_data.append(&mut profiling_data);
        self.steps += steps;
        Ok(())
    }

    pub fn write_frame(&mut self, time: Range<f64>) -> Result<(), ProfilerError> {
        let data = take(&mut self.buffered_data);
        let steps = take(&mut self.steps);

        let time_step = (time.end - time.start) / steps as f64;
        for (i, data) in data.chunks(data.len() / steps).enumerate() {
            let approx_step_time = time.start + time_step * i as f64;
            self.write_step(approx_step_time, data)?;
        }

        self.writer.flush()?;

        Ok(())
    }

    fn write_step(
        &mut self,
        time: f64,
        profiling_data: &[(String, Range<f64>)],
    ) -> Result<(), ProfilerError> {
        let labels: Vec<String> = profiling_data
            .iter()
            .map(|(label, _)| label.clone())
            .collect();
        if let Some(old_labels) = self.labels.as_ref() {
            if *old_labels != labels {
                return Err(ProfilerError::LayoutChanged {
                    old_labels: old_labels.clone(),
                    labels,
                });
            }
        } else {
            self.write_header(&labels)?;
            self.labels = Some(labels);
        }

        let offset = profiling_data
            .first()
            .map(|(_, range)| range.start)
            .ok_or(ProfilerError::ResultsEmpty)?;

        write!(self.writer, "{time}")?;
        for (_, range) in profiling_data {
            write!(
                self.writer,
                ",{:.3},{:.3}",
                (range.start - offset) * 1e3,
                (range.end - offset) * 1e3,
            )?;
        }
        writeln!(self.writer)?;

        Ok(())
    }

    fn write_header(&mut self, labels: &[String]) -> Result<(), ProfilerError> {
        write!(self.writer, "time")?;

        for label in labels {
            let name = gnuplot_name(label);
            write!(self.writer, ",{name},{name}-end")?;
        }

        writeln!(self.writer)?;

        Ok(())
    }
}

fn gnuplot_name(name: &str) -> String {
    let mut out = String::new();

    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            out.push('-');
        }
    }

    out
}
