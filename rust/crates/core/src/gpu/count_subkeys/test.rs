use wgpu::util::DeviceExt as _;

use super::*;

#[test]
fn test_simple() {
    let bit_count = 2;
    let bit_offset = 0;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 1);

    assert_eq!(
        count_subkeys_on_cpu(
            bit_count,
            bit_offset,
            workgroup_size,
            subgroup_size,
            &indices,
            &keys
        ),
        run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys,),
    );
}

#[test]
fn test_simple_with_offset() {
    let bit_count = 2;
    let bit_offset = 2;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 2);

    assert_eq!(
        count_subkeys_on_cpu(
            bit_count,
            bit_offset,
            workgroup_size,
            subgroup_size,
            &indices,
            &keys
        ),
        run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys),
    );
}

#[test]
fn test_larger() {
    let bit_count = 2;
    let bit_offset = 0;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys = [1; 513];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 3);

    assert_eq!(
        count_subkeys_on_cpu(
            bit_count,
            bit_offset,
            workgroup_size,
            subgroup_size,
            &indices,
            &keys
        ),
        run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let bit_count = 5;
    let workgroup_size = 64;
    let subgroup_size = get_subgroup_size();

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    shuffle(&mut indices, 4);

    for bit_offset in 0..5 {
        assert_eq!(
            count_subkeys_on_cpu(
                bit_count,
                bit_offset,
                workgroup_size,
                subgroup_size,
                &indices,
                &keys
            ),
            run_subkey_count(workgroup_size, bit_count, bit_offset, &indices, &keys),
        );
    }
}

fn run_subkey_count(
    workgroup_size: u32,
    bit_count: u32,
    bit_offset: u32,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let count_subkeys = CountSubkeys::new(&context, workgroup_size, bit_count);

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("indices"),
        contents: bytemuck::cast_slice(indices),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let key_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("keys"),
        contents: bytemuck::cast_slice(keys),
        usage: wgpu::BufferUsages::STORAGE,
    });

    let count_size = count_subkeys.min_counts(keys.len() as u32) * 4;

    let count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("counts"),
        size: count_size as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let download_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("download"),
        size: count_buffer.size(),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut compute_pass = encoder.begin_compute_pass(&Default::default());

    count_subkeys.compute_in_pass(
        &context,
        &mut compute_pass,
        index_buffer.as_entire_buffer_binding(),
        key_buffer.as_entire_buffer_binding(),
        count_buffer.as_entire_buffer_binding(),
        bit_offset,
    );

    drop(compute_pass);
    encoder.copy_buffer_to_buffer(&count_buffer, 0, &download_buffer, 0, None);

    context.queue().submit([encoder.finish()]);
    let data_buffer_slice = download_buffer.slice(..);
    data_buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let data = data_buffer_slice.get_mapped_range();
    let result: &[u32] = bytemuck::cast_slice(&data);

    result.to_vec()
}
