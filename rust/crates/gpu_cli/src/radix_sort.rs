use squishy_volumes_gpu::{
    GpuContext, MAX_NUM_PARTICLES, PipelinePart, RadixSort, RadixSortBufferInput, RadixSortSettings,
};

use crate::{Tool, window::run_with_window};

pub fn radix_sort_on_gpu(
    tool: Option<Tool>,
    settings: RadixSortSettings,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let radix_sort = RadixSort::new(&context, settings);

    let buffers = radix_sort.create_buffers(&context, RadixSortBufferInput { keys, indices });

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            radix_sort.compute_in_pass_all_rounds(
                context,
                &mut encoder.begin_compute_pass(&Default::default()),
                &mut (&buffers).into(),
            );
        });
        return Default::default();
    }

    let download_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_indices"),
        size: buffers.indices_back.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut buffer_bindings = (&buffers).into();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler = wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap();
    {
        let mut scope = profiler.scope("run_radix_sort", &mut encoder);
        let mut compute_pass = scope.scoped_compute_pass("pass");

        radix_sort.compute_in_pass_all_rounds(&context, &mut compute_pass, &mut buffer_bindings);
    };

    let last_index_buffer = if buffer_bindings.indices.swapped() {
        buffers.indices_front
    } else {
        buffers.indices_back
    };
    encoder.copy_buffer_to_buffer(&last_index_buffer, 0, &download_index_buffer, 0, None);

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let data_buffer_index_slice = download_index_buffer.slice(..);
    data_buffer_index_slice.map_async(wgpu::MapMode::Read, |_| {});
    profiler.end_frame().unwrap();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let profiling_data = profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .and_then(|data| data[0].nested_queries[0].time.clone())
        .map(|time| (time.end - time.start) * 1e6);
    tracing::info!(?profiling_data);
    println!("XXX: {}", profiling_data.unwrap());

    let data_indices = data_buffer_index_slice.get_mapped_range();
    let indices: &[u32] = bytemuck::cast_slice(&data_indices);

    indices.to_vec()
}
