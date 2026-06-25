use std::ops::Range;

use squishy_volumes_gpu::GpuContext;

pub fn profiler_output(context: &GpuContext, profiler: &mut wgpu_profiler::GpuProfiler) {
    let profiling_data: Vec<(String, Range<f64>)> = profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .unwrap()[0]
        .nested_queries
        .iter()
        .cloned()
        .map(|query| (query.label, query.time.unwrap()))
        .collect();
    let mut end = None;
    let mut total_duration = 0.;
    let mut total_inbetween = 0.;
    for (label, time) in profiling_data {
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
}
