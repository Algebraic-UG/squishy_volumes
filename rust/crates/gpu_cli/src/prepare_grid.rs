use nalgebra::Vector4;
use squishy_volumes_gpu::{
    DownloadsToHost, GpuContext, MAX_NUM_PARTICLES, PipelinePart, PrepareGrid,
    PrepareGridBufferInput, PrepareGridSettings, gpu_grid_to_cpu_grid,
};

use crate::{Tool, window::run_with_window};

pub fn prepare_grid_on_gpu(
    tool: Option<Tool>,
    settings: PrepareGridSettings,
    positions: &[Vector4<f32>],
    indices: &[u32],
) -> Vec<Vector4<i32>> {
    let context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    let device = context.device();

    let prepare_grid = PrepareGrid::new(&context, settings);
    let buffers =
        prepare_grid.create_buffers(&context, PrepareGridBufferInput { positions, indices });
    let downloads = DownloadsToHost::new(
        &context,
        [
            (&buffers.limits, "limits"),
            (&buffers.cell_ids_out, "cell_ids_out"),
            (&buffers.cell_owns, "cell_owns"),
        ],
    );

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            prepare_grid.compute_in_pass(
                context,
                &mut encoder.begin_compute_pass(&Default::default()),
                (&buffers).into(),
                (),
            );
        });
        return Default::default();
    }

    let mut encoder = context
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

    let mut profiler = wgpu_profiler::GpuProfiler::new(device, Default::default()).unwrap();
    {
        let mut scope = profiler.scope("run_prepare_grid", &mut encoder);
        let mut compute_pass = scope.scoped_compute_pass("pass");

        prepare_grid.compute_in_pass(&context, &mut compute_pass, (&buffers).into(), ());
    }

    downloads.copy(&mut encoder);
    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    profiler.end_frame().unwrap();

    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();

    let profiling_data = profiler
        .process_finished_frame(context.queue().get_timestamp_period())
        .and_then(|data| data[0].nested_queries[0].time.clone())
        .map(|time| (time.end - time.start) * 1e6);
    tracing::info!(?profiling_data);
    println!("XXX: {}", profiling_data.unwrap());

    let [limits, cell_ids_out, cell_owns] = downloads.try_into().unwrap();

    gpu_grid_to_cpu_grid(
        &limits.to_vec(),
        &cell_ids_out.to_vec(),
        &cell_owns.to_vec(),
    )
}
