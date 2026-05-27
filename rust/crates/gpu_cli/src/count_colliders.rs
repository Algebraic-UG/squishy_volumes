use nalgebra::Vector4;
use squishy_volumes_gpu::{
    GpuContext, PipelinePart, Triangle, build_hash_table_on_cpu_simple, count_colliders::*,
};

use crate::{Tool, window::run_with_window};

pub fn count_colliders_on_gpu(tool: Option<Tool>, _settings: Settings) {
    let mut context = GpuContext::new().unwrap();
    context.setup_allocator(1 << 15, "allocator", true).unwrap();
    context
        .setup_indirect_allocator(400, "indirect allocator", true)
        .unwrap();

    // From test
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        cell_size: 0.5,
        layers: 3,
    };
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];
    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&[Vector4::new(4, -1, 0, 0)]);
    let input_data = InputData {
        collider_meshes: vec![(&vertices, &triangles)],
        block_ids: &block_ids,
        block_table: &block_table,
    };
    // From test

    let count_colliders = CountColliders::new(&context, settings);
    let input = Input::new(context.device(), settings, input_data);

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            count_colliders
                .record(context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
    } else {
        todo!()
    }
}
