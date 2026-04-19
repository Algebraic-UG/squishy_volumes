use squishy_volumes_gpu::{
    DownloadToHost, GpuContext, MAX_NUM_PARTICLES, PipelinePart, radix_sort::*,
};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn radix_sort_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let mut context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    context
        .setup_allocator(indices.len() as u64 * 10, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(400, "indirect allocator", true)
        .unwrap();

    let radix_sort = RadixSort::new(&context, settings.clone());

    let input = Input::new(context.device(), settings, Some(indices), keys);

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            radix_sort
                .record_all_rounds(context, &mut encoder.into(), input)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_radix_sort", &mut encoder);
    let indices_out = radix_sort
        .record_all_rounds(&mut context, &mut scope.into(), input)
        .unwrap();

    let download = DownloadToHost::new(&context, indices_out);
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
