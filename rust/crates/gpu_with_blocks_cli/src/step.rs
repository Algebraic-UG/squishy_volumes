use nalgebra::{Matrix1x3, Matrix3, Matrix4x3, Vector4, stack};
use squishy_volumes_gpu::{
    DownloadToHost, GpuContext, PipelinePart,
    particle_parameters::{Host, Solid},
    step::*,
};
use squishy_volumes_util::triangle::{Opposites, Triangle};

use crate::{Tool, profiler_output::profiler_output, window::run_with_window};

pub fn step_on_gpu(
    tool: Option<Tool>,
    settings: Settings,
    positions: &[Vector4<f32>],
) -> Vec<Vector4<f32>> {
    let mut context = GpuContext::new().unwrap();
    context
        .setup_allocator(positions.len() as u64 * 1024, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(2048, "indirect allocator", true)
        .unwrap();

    let step = Step::new(&context, settings.clone());
    let n = positions.len();
    let input = Input::new(
        context.device(),
        1., // leaf_size
        16, // leaf_threshold
        settings,
        InputData {
            indices: &vec![0; n],
            collider_bits: &vec![0; n],
            masses: &vec![1.; n],
            initial_volumes: &vec![1.; n],
            parameters: &vec![
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

            vertex_positions_start: &[
                Vector4::new(1., 1., 1., 0.),
                Vector4::new(0., 1., 0., 0.),
                Vector4::new(1., 0., 0., 0.),
            ],
            vertex_positions_end: &[
                Vector4::new(1., 1., 1., 0.),
                Vector4::new(0., 1., 0., 0.),
                Vector4::new(1., 0., 0., 0.),
            ],
            triangle_indices: &[Triangle { a: 0, b: 1, c: 1 }],
            triangle_collider: &[0],
            triangle_opposites: &[Opposites {
                ab: u32::MAX,
                bc: u32::MAX,
                ca: u32::MAX,
            }],
            triangle_frictions: &[0.],
        },
    );

    let paramters = Parameters { factor: 0. };

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            step.record(context, &mut encoder.into(), input, paramters)
                .unwrap();
        });
        return Default::default();
    }

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run_step", &mut encoder);
    let Output { positions_out, .. } = step
        .record(&mut context, &mut scope.into(), input, paramters)
        .unwrap();

    let download = DownloadToHost::new(&context, positions_out);
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
