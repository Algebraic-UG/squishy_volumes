use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector4, stack};
use squishy_volumes_gpu::{
    DownloadToHost, GpuContext, MAX_NUM_PARTICLES, PipelinePart,
    collect::*,
    particle_parameters::{Host, Solid},
    scatter::InputData,
};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn collect_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    positions: &[Vector4<f32>],
) -> Vec<Vector4<f32>> {
    let mut context = GpuContext::new(MAX_NUM_PARTICLES).unwrap();
    context
        .setup_allocator(positions.len() as u64 * 165, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(400, "indirect allocator", true)
        .unwrap();

    let collect = Collect::new(&context, settings);
    let n = positions.len();
    let input = Input::new(
        context.device(),
        settings,
        (u16::MAX as u32).try_into().unwrap(),
        context.subgroup_size(),
        InputData {
            masses: &vec![1.; n],
            initial_volumes: &vec![1.; n],
            particle_parameters: &vec![
                Host::Solid(Solid {
                    mu: 1.,
                    lambda: 1.,
                    viscosity: None,
                    sand_alpha: None,
                })
                .into();
                n
            ],
            positions,
            position_gradients: &vec![
                stack![
                    Matrix3::identity();
                    Matrix1x3::zeros()
                ];
                n
            ],
            velocities: &vec![Vector4::zeros(); n],
            velocity_gradients: &vec![Matrix4x3::zeros(); n],
        },
    );

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            collect
                .record(context, &mut encoder.into(), input, Parameters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_collect", &mut encoder);
    let Output { positions, .. } = collect
        .record(&mut context, &mut scope.into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, positions);
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
