use std::ops::Range;

use squishy_volumes_gpu::{
    DownloadToHost, GpuContext, MAX_NUM_PARTICLES, PipelinePart, prefix_sum::*,
};

use crate::{Tool, window::run_with_window};

pub fn prefix_sum_on_gpu(tool: Option<Tool>, settings: Settings, numbers: &[u32]) -> Vec<u32> {
    let mut context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    context
        .setup_allocator(numbers.len() as u64 * 5, "allocator")
        .unwrap();
    context
        .setup_indirect_allocator(100, "indirect allocator")
        .unwrap();

    let prefix_sum = PrefixSum::new(&context, settings);

    let input = Input::new(context.device(), settings, numbers);

    if let Some(tool) = tool {
        run_with_window(tool, context, |mut context, encoder| {
            prefix_sum
                .record(&mut context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let Output { prefix_sums } = {
        let scope = profiler.scope("run_prefix_sum", &mut encoder);
        prefix_sum
            .record(&mut context, &mut scope.into(), input, Parameters)
            .unwrap()
    };

    let download = DownloadToHost::new(&context, prefix_sums);
    download.copy(&mut encoder);

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    profiler.end_frame().unwrap();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

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

    download.to_vec()
}
