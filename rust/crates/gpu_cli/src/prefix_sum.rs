use squishy_volumes_gpu::{
    GpuContext, MAX_NUM_PARTICLES, PipelinePart, PrefixSum, PrefixSumBufferInput, PrefixSumSettings,
};

use crate::{Tool, window::run_with_window};

pub fn prefix_sum_on_gpu(
    tool: Option<Tool>,
    settings: PrefixSumSettings,
    input: &[u32],
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let prefix_sum = PrefixSum::new(&context, settings);
    let buffers = prefix_sum.create_buffers(&context, PrefixSumBufferInput { numbers: input });

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            prefix_sum.compute_in_pass(
                context,
                &mut encoder.begin_compute_pass(&Default::default()),
                &mut (&buffers).into(),
                &mut (),
            );
        });
        return Default::default();
    }

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.prefix_sums.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut profiler = wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap();
    {
        let mut scope = profiler.scope("run_prefix_sum", &mut encoder);
        let mut compute_pass = scope.scoped_compute_pass("pass");

        prefix_sum.compute_in_pass(&context, &mut compute_pass, &mut (&buffers).into(), &mut ());
    }

    encoder.copy_buffer_to_buffer(&buffers.prefix_sums, 0, &download_buffer, 0, None);

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    profiler.end_frame().unwrap();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let profiling_data = profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .and_then(|data| data[0].nested_queries[0].time.clone())
        .map(|time| (time.end - time.start) * 1e6);
    tracing::info!(?profiling_data);
    println!("XXX: {}", profiling_data.unwrap());

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
