use squishy_volumes_gpu::{
    DoubleBuffer, GpuContext, MAX_NUM_PARTICLES, RadixSort, RadixSortBufferBindings,
    RadixSortSettings,
};
use wgpu::util::DeviceExt as _;

use crate::{Tool, window::run_with_window};

pub fn radix_sort_on_gpu(
    tool: Option<Tool>,
    settings: RadixSortSettings,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let prefix_sort = RadixSort::new(&context, settings);

    let key_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("keys"),
        contents: bytemuck::cast_slice(keys),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let index_buffer_front = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("index_front"),
        contents: bytemuck::cast_slice(&indices),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
    });
    let index_buffer_back = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("index_back"),
        size: index_buffer_front.size(),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let count_size = prefix_sort.min_counts(keys.len() as u32) * 4;
    let prefix_size = prefix_sort.min_prefixes(keys.len() as u32) * 4;

    let count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("count"),
        size: count_size as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let prefix_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("prefix"),
        size: prefix_size as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });

    let index_buffers = DoubleBuffer::new(
        index_buffer_front.as_entire_buffer_binding(),
        index_buffer_back.as_entire_buffer_binding(),
    );

    let mut radix_sort_buffers = RadixSortBufferBindings {
        keys: key_buffer.as_entire_buffer_binding(),
        indices: index_buffers,
        counts: count_buffer.as_entire_buffer_binding(),
        prefixes: prefix_buffer.as_entire_buffer_binding(),
    };

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            prefix_sort.compute_in_pass(
                context,
                &mut encoder.begin_compute_pass(&Default::default()),
                &mut radix_sort_buffers,
            );
        });
        return Default::default();
    }

    let download_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_indices"),
        size: index_buffer_front.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler = wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap();
    {
        let mut scope = profiler.scope("run_prefix_sort", &mut encoder);
        let mut compute_pass = scope.scoped_compute_pass("pass");

        prefix_sort.compute_in_pass(&context, &mut compute_pass, &mut radix_sort_buffers);
    };

    let last_index_buffer = if radix_sort_buffers.indices.swapped() {
        index_buffer_front
    } else {
        index_buffer_back
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
