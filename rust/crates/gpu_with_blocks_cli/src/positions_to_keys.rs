use nalgebra::Vector4;
use squishy_volumes_gpu::{DownloadToHost, GpuContext, PipelinePart, positions_to_keys::*};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn positions_to_keys_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    parameters: Parameters,
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let mut context = GpuContext::new().unwrap();
    context
        .setup_allocator(positions.len() as u64 * 4, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(100, "indirect allocator", true)
        .unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        (u16::MAX as u32).try_into().unwrap(),
        positions,
    );
    let positions_to_keys = PositionsToKeys::new(&context, settings);

    if let Some(tool) = tool {
        run_with_window(tool, context, |mut context, encoder| {
            positions_to_keys
                .record(&mut context, &mut encoder.into(), input, parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_positions_to_keys", &mut encoder);
    let Output { keys } = positions_to_keys
        .record(&mut context, &mut scope.into(), input, parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, keys);
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
