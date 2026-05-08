use nalgebra::Vector4;
use squishy_volumes_gpu::{DownloadToHost, GpuContext, PipelinePart, build_hash_table::*};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn build_hash_table_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    cells: &[Vector4<i32>],
) -> Vec<u32> {
    let mut context = GpuContext::new().unwrap();
    context
        .setup_allocator(cells.len() as u64 * 128, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(400, "indirect allocator", true)
        .unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        (u16::MAX as u32).try_into().unwrap(),
        cells,
    );

    let build_hash_table = BuildHashTable::new(&context, settings);

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            build_hash_table
                .record(context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_build_hash_table", &mut encoder);
    let Output { block_table, .. } = build_hash_table
        .record(&mut context, &mut scope.into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, block_table);
    download.copy(&mut encoder);

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    profiler.end_frame().unwrap();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    profiler_output(&context, &mut profiler);
    download.to_vec()
}
