use nalgebra::Vector4;
use squishy_volumes_gpu::{
    Block, DownloadToHost, GpuContext, MAX_NUM_PARTICLES, PipelinePart, scatter::*,
};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn scatter_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    positions: &[Vector4<f32>],
) -> Vec<Block> {
    let mut context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    context
        .setup_allocator(positions.len() as u64 * 200, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(400, "indirect allocator", true)
        .unwrap();

    let scatter = Scatter::new(&context, settings);
    let (input, addendum) = Input::new(
        context.device(),
        settings,
        (u16::MAX as u32).try_into().unwrap(),
        context.subgroup_size(),
        positions,
    );

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            scatter
                .record(context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_scatter", &mut encoder);
    let Output { blocks } = scatter
        .record(&mut context, &mut scope.into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, blocks);
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
