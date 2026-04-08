use nalgebra::Vector4;
use squishy_volumes_gpu::{
    GpuContext, MAX_NUM_PARTICLES, PipelinePart, SortPositionsIntoCells,
    SortPositionsIntoCellsBufferInput, SortPositionsIntoCellsSettings,
};

use crate::{Tool, window::run_with_window};

pub fn sort_positions_into_cells_on_gpu(
    tool: Option<Tool>,
    settings: SortPositionsIntoCellsSettings,
    indices: &[u32],
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let sort_positions_into_cells = SortPositionsIntoCells::new(&context, settings);
    let buffers = sort_positions_into_cells.create_buffers(
        &context,
        SortPositionsIntoCellsBufferInput { indices, positions },
    );

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            sort_positions_into_cells.compute_in_pass(
                context,
                &mut encoder.begin_compute_pass(&Default::default()),
                &(&buffers).into(),
                &(),
            );
        });
        return Default::default();
    }

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: buffers.radix_sort.indices_back.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let buffer_bindings = (&buffers).into();

    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut profiler = wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap();
    {
        let mut scope = profiler.scope("run_sort_positions_into_cells", &mut encoder);
        let mut compute_pass = scope.scoped_compute_pass("pass");

        sort_positions_into_cells.compute_in_pass(
            &context,
            &mut compute_pass,
            &buffer_bindings,
            &(),
        );
    }

    encoder.copy_buffer_to_buffer(
        buffer_bindings.radix_sort.indices.front().buffer,
        0,
        &download_buffer,
        0,
        None,
    );

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
