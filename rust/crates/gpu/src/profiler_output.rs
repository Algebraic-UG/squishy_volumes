use std::ops::Range;
use std::path::Path;
use std::{
    fs::File,
    io::{self, BufWriter, Write},
};

use super::GpuContext;

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

    #[error("IoError: {0}")]
    IoError(#[from] io::Error),
}

fn get_profiling_data(
    context: &GpuContext,
    profiler: &mut wgpu_profiler::GpuProfiler,
) -> Result<Vec<(String, Range<f64>)>, ProfilerError> {
    profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .ok_or(ProfilerError::NoFinishedFrame)?
        .first()
        .ok_or(ProfilerError::ResultsEmpty)?
        .nested_queries
        .iter()
        .cloned()
        .map(|query| {
            Ok((
                query.label,
                query.time.ok_or(ProfilerError::NoTimeRecorded)?,
            ))
        })
        .collect()
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

#[derive(Default)]
pub struct ProfileDataCsvWriter<W> {
    writer: W,
    labels: Option<Vec<String>>,
}

impl ProfileDataCsvWriter<BufWriter<File>> {
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
        frame: usize,
    ) -> Result<(), ProfilerError> {
        let profiling_data = get_profiling_data(context, profiler)?;

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

        write!(self.writer, "{frame}")?;
        for (_, range) in profiling_data {
            write!(
                self.writer,
                ",{:.3},{:.3}",
                (range.start - offset) * 1e3,
                (range.end - offset) * 1e3,
            )?;
        }
        writeln!(self.writer)?;
        self.writer.flush()?;

        Ok(())
    }

    fn write_header(&mut self, labels: &[String]) -> Result<(), ProfilerError> {
        write!(self.writer, "frame")?;

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
