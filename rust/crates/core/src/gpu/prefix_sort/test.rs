use wgpu::util::DeviceExt as _;

use super::*;

#[test]
fn test_simple() {
    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_prefix_sort(
            PrefixSortSettings {
                prefix_sum_workgroup_size: 64,
                count_subkeys_workgroup_size: 64,
                reorder_workgroup_size: 64,
                bit_count: 2,
            },
            &indices,
            &keys,
        )
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_prefix_sort(
            PrefixSortSettings {
                prefix_sum_workgroup_size: 64,
                count_subkeys_workgroup_size: 64,
                reorder_workgroup_size: 64,
                bit_count: 2,
            },
            &indices,
            &keys,
        )
    );
}

fn run_prefix_sort(settings: PrefixSortSettings, indices: &[u32], keys: &[u32]) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let prefix_sort = PrefixSort::new(&context, settings);

    let key_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("keys"),
        contents: bytemuck::cast_slice(keys),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let index_buffer_front = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("index_front"),
        contents: bytemuck::cast_slice(indices),
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

    let download_index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download_indices"),
        size: index_buffer_front.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let index_buffers = DoubleBuffer::new(
        index_buffer_front.as_entire_buffer_binding(),
        index_buffer_back.as_entire_buffer_binding(),
    );

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    let swapped = prefix_sort.compute_in_pass(
        &context,
        &mut compute_pass,
        PrefixSortBufferBindings {
            keys: key_buffer.as_entire_buffer_binding(),
            indices: index_buffers,
            counts: count_buffer.as_entire_buffer_binding(),
            prefixes: prefix_buffer.as_entire_buffer_binding(),
        },
    );

    drop(compute_pass);
    let last_index_buffer = if swapped {
        index_buffer_front
    } else {
        index_buffer_back
    };
    encoder.copy_buffer_to_buffer(&last_index_buffer, 0, &download_index_buffer, 0, None);

    context.queue().submit([encoder.finish()]);

    let data_buffer_index_slice = download_index_buffer.slice(..);
    data_buffer_index_slice.map_async(wgpu::MapMode::Read, |_| {});

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data_indices = data_buffer_index_slice.get_mapped_range();
    let indices: &[u32] = bytemuck::cast_slice(&data_indices);

    indices.to_vec()
}
