// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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

    #[error("IoError: {0}")]
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
    writer: BufWriter<File>,
    labels: Option<Vec<String>>,
}

impl ProfileDataCsvWriter {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ProfilerError> {
        let writer = BufWriter::new(File::create(path)?);
        Ok(Self {
            writer,
            labels: None,
        })
    }

    pub fn write_frame(
        &mut self,
        context: &GpuContext,
        profiler: &mut wgpu_profiler::GpuProfiler,
        times: &[f64],
    ) -> Result<(), ProfilerError> {
        let profiling_data = get_profiling_data(context, profiler)?;
        if profiling_data.is_empty() {
            return Err(ProfilerError::ResultsEmpty);
        }

        if !profiling_data.len().is_multiple_of(times.len()) {
            return Err(ProfilerError::NotMultipleOfRecordedSteps {
                steps: times.len(),
                labels: profiling_data
                    .into_iter()
                    .map(|label_and_range| label_and_range.0)
                    .collect(),
            });
        }

        for (&time, profiling_data) in times
            .iter()
            .zip(profiling_data.chunks(profiling_data.len() / times.len()))
        {
            self.write_step(time, profiling_data)?;
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
