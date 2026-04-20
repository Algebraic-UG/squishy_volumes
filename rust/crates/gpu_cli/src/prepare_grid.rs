use nalgebra::Vector4;
use squishy_volumes_gpu::{
    DownloadsToHost, GpuContext, MAX_NUM_PARTICLES, PipelinePart, gpu_grid_to_cpu_grid,
    prepare_grid::*,
};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn prepare_grid_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    positions: &[Vector4<f32>],
) -> Vec<Vector4<i32>> {
    let mut context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    context
        .setup_allocator(positions.len().max(100) as u64 * 128, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(1028, "indirect allocator", true)
        .unwrap();

    let input = Input::new(context.device(), settings.clone(), positions);

    let prepare_grid = PrepareGrid::new(&context, settings);

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            prepare_grid
                .record(context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_prepare_grid", &mut encoder);
    let Output {
        indirect_cells,
        cell_ids,
        cell_owns,
        ..
    } = prepare_grid
        .record(&mut context, &mut scope.into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [indirect_cells, cell_ids, cell_owns]);
    downloads.copy(&mut encoder);

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    profiler.end_frame().unwrap();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    profiler_output(&context, &mut profiler);
    let [indirect_cells, cell_ids, cell_owns] = downloads.try_into().unwrap();

    gpu_grid_to_cpu_grid(
        indirect_cells.to_vec()[0],
        &cell_ids.to_vec(),
        &cell_owns.to_vec(),
    )
}
