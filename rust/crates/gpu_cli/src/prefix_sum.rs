use squishy_volumes_gpu::{
    DownloadToHost, GpuAllocator, GpuContext, MAX_NUM_PARTICLES, PipelinePart, prefix_sum::*,
};

use crate::{Tool, window::run_with_window};

pub fn prefix_sum_on_gpu(tool: Option<Tool>, settings: Settings, numbers: &[u32]) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let mut allocator =
        GpuAllocator::new(&context, numbers.len() as u64 * 5, "test allocator").unwrap();
    let device = context.device();

    let prefix_sum = PrefixSum::new(&context, settings);

    let standalone::Allocations { numbers, indirect } =
        standalone::Allocations::new(device, settings, numbers);

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            prefix_sum
                .record(
                    context,
                    &mut allocator,
                    &mut encoder.begin_compute_pass(&Default::default()),
                    InputBindings { indirect, numbers },
                    Parameters,
                )
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut profiler = wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap();
    let OutputBindings { prefix_sums } = {
        let mut scope = profiler.scope("run_prefix_sum", &mut encoder);
        let mut compute_pass = scope.scoped_compute_pass("pass");

        prefix_sum
            .record(
                &context,
                &mut allocator,
                &mut compute_pass,
                InputBindings { indirect, numbers },
                Parameters,
            )
            .unwrap()
    };

    let download = DownloadToHost::new(&context, prefix_sums);
    download.copy(&mut encoder);

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    profiler.end_frame().unwrap();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let profiling_data = profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .and_then(|data| data[0].nested_queries[0].time.clone())
        .map(|time| (time.end - time.start) * 1e6);
    tracing::info!(?profiling_data);
    println!("XXX: {}", profiling_data.unwrap());

    download.to_vec()
}
