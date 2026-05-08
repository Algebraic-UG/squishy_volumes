use nalgebra::Vector4;
use squishy_volumes_gpu::{DownloadToHost, GpuContext, PipelinePart, sort_positions_into_cells::*};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn sort_positions_into_cells_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let mut context = GpuContext::new().unwrap();
    context
        .setup_allocator(positions.len() as u64 * 16, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(400, "indirect allocator", true)
        .unwrap();

    let sort_positions_into_cells = SortPositionsIntoCells::new(&context, settings);
    let input = Input::new(context.device(), settings, positions);

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            sort_positions_into_cells
                .record(context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_positions_into_cells", &mut encoder);
    let Output { permutation } = sort_positions_into_cells
        .record(&mut context, &mut scope.into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, permutation);
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
