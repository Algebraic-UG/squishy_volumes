use std::num::NonZeroU32;

use gpu::{GpuContext, PipelinePart, profiler_output};
use nalgebra::{Vector3, Vector4};
use rand::{RngExt, SeedableRng, rngs::ChaCha8Rng};
use squishy_volumes_gpu::{
    self as gpu, contributors_on_cpu, get_node_set, prepare_tmp_on_cpu,
    test_data::{ParticleSampling, TestMesh, TestParticles},
};
use squishy_volumes_util::Aabb;
use tracing::dispatcher::set_global_default;
use tracing_subscriber::FmtSubscriber;

use clap::{Parser, ValueEnum};

use crate::window::run_with_window;

mod window;

#[derive(ValueEnum, Clone, Copy, Default, Debug)]
enum Tool {
    #[default]
    RenderDoc,
    Nsight,
}

#[derive(Debug, ValueEnum, Clone)]
enum Task {
    Sum,
    AnimateMesh,
    Collide,
    PartitionNodes,
    PrepareGrid,
    RegisterContributors,
    PrepareTmp,
    Scatter,
    MeldGrid,
    Collect,
}

#[derive(Parser)]
struct Cli {
    #[arg(value_enum)]
    task: Task,

    #[arg(long, value_name = "generate given amount of input")]
    generate: u32,

    #[arg(long, value_enum)]
    tool: Option<Tool>,

    #[arg(long, default_value_t = NonZeroU32::new(64).unwrap())]
    workgroup_size: NonZeroU32,

    #[arg(long, default_value_t = NonZeroU32::new(u16::MAX as u32).unwrap())]
    dispatch_limit: NonZeroU32,

    #[arg(long, default_value_t = 1234)]
    seed: u64,
}

fn main() {
    set_global_default(FmtSubscriber::default().into()).unwrap();

    let Cli {
        task,
        generate,
        tool,
        workgroup_size,
        dispatch_limit,
        seed,
    } = Cli::parse();

    let grid_node_size = 1.;
    let time_step = 0.001;
    let forget_distance = grid_node_size * 2.2;
    let accept_distance = grid_node_size * 2.;
    let leaf_size = accept_distance;
    let leaf_threshold = 16;

    let context = GpuContext::new().unwrap();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    match task {
        Task::Sum => {
            let numbers: Vec<u32> = (0..generate).map(|_| rng.random_range(0..100)).collect();
            let settings = gpu::prefix_sum::Settings {
                workgroup_size,
                dispatch_limit,
            };
            let pipeline_part = gpu::PrefixSum::new(&context, settings);
            let input = gpu::prefix_sum::Input::new(context.device(), settings, &numbers);
            run_pipeline_part(
                context,
                generate as u64 * 16,
                tool,
                pipeline_part,
                input,
                gpu::prefix_sum::Parameters { total_sum: true },
            );
        }
        Task::AnimateMesh => {
            let test_mesh = TestMesh::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-10.),
                    max: Vector3::repeat(10.),
                },
            );
            let settings = gpu::animate_mesh::Settings {
                workgroup_size,
                dispatch_limit,
            };
            let pipeline_part = gpu::AnimateMesh::new(&context, settings);
            let input = gpu::animate_mesh::Input::new(
                context.device(),
                gpu::animate_mesh::InputData {
                    vertex_positions_start: &test_mesh.vertex_positions_a,
                    vertex_positions_end: &test_mesh.vertex_positions_b,
                    triangle_indices: &test_mesh.triangle_indices,
                },
            );
            run_pipeline_part(
                context,
                generate as u64 * 1024,
                tool,
                pipeline_part,
                input,
                gpu::animate_mesh::Parameters { factor: 0.5 },
            );
        }
        Task::Collide => {
            let aabb = Aabb {
                min: Vector3::repeat(-10.),
                max: Vector3::repeat(10.),
            };
            let test_particles = TestParticles::new(
                generate as usize,
                aabb,
                ParticleSampling::Neat(grid_node_size / 10.),
            );
            let test_mesh = TestMesh::new(10000, aabb);
            let settings = gpu::collide::Settings {
                workgroup_size,
                dispatch_limit,
                forget_distance,
                accept_distance,
                time_step,
            };
            let pipeline_part = gpu::Collide::new(&context, settings);
            let input = gpu::collide::Input::new(
                context.device(),
                &settings,
                gpu::collide::InputData {
                    leaf_size,
                    leaf_threshold,
                    particle_positions_and_collider_bits: &test_particles
                        .particle_positions_and_collider_bits,
                    particle_velocities: &test_particles.particle_velocities,
                    vertex_positions: &test_mesh.vertex_positions_a,
                    vertex_normals: &test_mesh.vertex_normals_a,
                    triangle_indices: &test_mesh.triangle_indices,
                    triangle_collider: &vec![0; test_mesh.triangle_indices.len()],
                    triangle_normals: &test_mesh.triangle_normals_a,
                    triangle_opposites: &test_mesh.triangle_opposites,
                    triangle_frictions: &test_mesh.triangle_frictions_a,
                },
            );
            run_pipeline_part(
                context,
                generate as u64,
                tool,
                pipeline_part,
                input,
                gpu::collide::Parameters,
            );
        }
        Task::PartitionNodes => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-1000.),
                    max: Vector3::repeat(1000.),
                },
                ParticleSampling::Neat(grid_node_size / 2.),
            );
            let settings = gpu::partition_nodes::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
            };
            let pipeline_part = gpu::PartitionNodes::new(&context, settings.clone());
            let input = gpu::partition_nodes::Input::new(
                context.device(),
                &test_particles.particle_positions_and_collider_bits,
            );
            run_pipeline_part(
                context,
                generate as u64 * 1024,
                tool,
                pipeline_part,
                input,
                gpu::partition_nodes::Parameters,
            );
        }
        Task::PrepareGrid => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-1000.),
                    max: Vector3::repeat(1000.),
                },
                ParticleSampling::Neat(grid_node_size / 2.),
            );
            let settings = gpu::prepare_grid::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
            };
            let pipeline_part = gpu::PrepareGrid::new(&context, settings.clone());
            let input = gpu::prepare_grid::Input::new(
                context.device(),
                settings,
                &test_particles.particle_positions_and_collider_bits,
            );
            run_pipeline_part(
                context,
                generate as u64 * 2048,
                tool,
                pipeline_part,
                input,
                gpu::prepare_grid::Parameters,
            );
        }
        Task::RegisterContributors => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-1000.),
                    max: Vector3::repeat(1000.),
                },
                ParticleSampling::Neat(grid_node_size / 2.),
            );
            tracing::info!("creating nodes");
            let node_ids_and_collider_bits: Vec<_> = get_node_set(
                grid_node_size,
                &test_particles.particle_positions_and_collider_bits,
            )
            .into_iter()
            .collect();
            let settings = gpu::register_contributors::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
            };
            let pipeline_part = gpu::RegisterContributors::new(&context, settings.clone());
            tracing::info!("creating input");
            let input = gpu::register_contributors::Input::new(
                context.device(),
                settings,
                &node_ids_and_collider_bits,
                &test_particles.particle_positions_and_collider_bits,
            );
            run_pipeline_part(
                context,
                generate as u64 * 2048,
                tool,
                pipeline_part,
                input,
                gpu::register_contributors::Parameters,
            );
        }
        Task::PrepareTmp => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-10.),
                    max: Vector3::repeat(10.),
                },
                ParticleSampling::Random,
            );
            let settings = gpu::prepare_tmp::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
                time_step,
            };
            let pipeline_part = gpu::PrepareTmp::new(&context, settings);
            let input = gpu::prepare_tmp::Input::new(
                context.device(),
                gpu::prepare_tmp::InputData {
                    particle_masses: &test_particles.particle_masses,
                    particle_initial_volumes: &test_particles.particle_initial_volumes,
                    particle_parameters: &test_particles.particle_parameters,
                    particle_positions_and_collider_bits: &test_particles
                        .particle_positions_and_collider_bits,
                    particle_position_gradients: &test_particles.particle_position_gradients,
                    particle_velocities: &test_particles.particle_velocities,
                    particle_velocity_gradients: &test_particles.particle_velocity_gradients,
                },
            );
            run_pipeline_part(
                context,
                generate as u64 * 16 * 4,
                tool,
                pipeline_part,
                input,
                gpu::prepare_tmp::Parameters,
            );
        }
        Task::Scatter => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-1000.),
                    max: Vector3::repeat(1000.),
                },
                ParticleSampling::Neat(grid_node_size / 2.),
            );
            let (node_ids_and_collider_bits, contributor_offsets, contributors) =
                contributors_on_cpu(
                    grid_node_size,
                    &test_particles.particle_positions_and_collider_bits,
                );
            let particle_tmp = prepare_tmp_on_cpu(
                grid_node_size,
                time_step,
                gpu::prepare_tmp::InputData {
                    particle_masses: &test_particles.particle_masses,
                    particle_initial_volumes: &test_particles.particle_initial_volumes,
                    particle_parameters: &test_particles.particle_parameters,
                    particle_positions_and_collider_bits: &test_particles
                        .particle_positions_and_collider_bits,
                    particle_position_gradients: &test_particles.particle_position_gradients,
                    particle_velocities: &test_particles.particle_velocities,
                    particle_velocity_gradients: &test_particles.particle_velocity_gradients,
                },
            );

            let settings = gpu::scatter::Settings {
                workgroup_size,
                grid_node_size,
            };
            let pipeline_part = gpu::Scatter::new(&context, settings);
            let input = gpu::scatter::Input::new(
                context.device(),
                settings,
                dispatch_limit,
                gpu::scatter::InputData {
                    contributor_offsets: &contributor_offsets,
                    contributors: &contributors,
                    node_ids_and_collider_bits: &node_ids_and_collider_bits,
                    particle_tmp: &particle_tmp,
                },
            );
            run_pipeline_part(
                context,
                generate as u64 * 4 * 4 * 27,
                tool,
                pipeline_part,
                input,
                gpu::scatter::Parameters,
            );
        }
        Task::MeldGrid => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-1000.),
                    max: Vector3::repeat(1000.),
                },
                ParticleSampling::Neat(grid_node_size / 2.),
            );
            let node_ids_and_collider_bits: Vec<_> = get_node_set(
                grid_node_size,
                &test_particles.particle_positions_and_collider_bits,
            )
            .into_iter()
            .collect();
            let node_momentums_in: Vec<_> = (0..node_ids_and_collider_bits.len())
                .map(|_| {
                    Vector4::new(
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(0.1..10.),
                    )
                })
                .collect();

            let settings = gpu::meld_grid::Settings { workgroup_size };
            let pipeline_part = gpu::MeldGrid::new(&context, settings.clone());
            let input = gpu::meld_grid::Input::new(
                context.device(),
                settings,
                dispatch_limit,
                gpu::meld_grid::InputData {
                    node_ids_and_collider_bits: &node_ids_and_collider_bits,
                    node_momentums_in: &node_momentums_in,
                },
            );
            run_pipeline_part(
                context,
                generate as u64 * 4 * 4 * 27,
                tool,
                pipeline_part,
                input,
                gpu::meld_grid::Parameters,
            );
        }
        Task::Collect => {
            let test_particles = TestParticles::new(
                generate as usize,
                Aabb {
                    min: Vector3::repeat(-1000.),
                    max: Vector3::repeat(1000.),
                },
                ParticleSampling::Neat(grid_node_size / 2.),
            );
            let node_ids_and_collider_bits: Vec<_> = get_node_set(
                grid_node_size,
                &test_particles.particle_positions_and_collider_bits,
            )
            .into_iter()
            .collect();
            let node_momentums: Vec<_> = (0..node_ids_and_collider_bits.len())
                .map(|_| {
                    Vector4::new(
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(-1.0..1.),
                        rng.random_range(0.1..10.),
                    )
                })
                .collect();

            let settings = gpu::collect::Settings {
                workgroup_size,
                dispatch_limit,
                grid_node_size,
                time_step,
            };
            let pipeline_part = gpu::Collect::new(&context, settings);
            let input = gpu::collect::Input::new(
                context.device(),
                gpu::collect::InputData {
                    node_ids_and_collider_bits: &node_ids_and_collider_bits,
                    node_momentums: &node_momentums,
                    particle_positions_and_collider_bits: &test_particles
                        .particle_positions_and_collider_bits,
                    particle_position_gradients: &test_particles.particle_position_gradients,
                    particle_velocities: &test_particles.particle_velocities,
                    particle_velocity_gradients: &test_particles.particle_velocity_gradients,
                },
            );
            run_pipeline_part(
                context,
                0,
                tool,
                pipeline_part,
                input,
                gpu::collect::Parameters,
            );
        }
    };
}

fn run_pipeline_part<P: PipelinePart>(
    mut context: GpuContext,
    allocator_size: u64,
    tool: Option<Tool>,
    pipeline_part: P,
    input: P::Input,
    parameters: P::Parameters,
) {
    tracing::info!("setting up allocator");
    context
        .setup_allocator(allocator_size, "allocator", true)
        .unwrap();
    context
        .setup_indirect_allocator(2048, "indirect allocator", true)
        .unwrap();

    if let Some(tool) = tool {
        run_with_window(tool, context, |context, encoder| {
            pipeline_part
                .record(context, &mut encoder.into(), input, parameters)
                .unwrap();
        });
        return;
    }

    tracing::info!("recording");
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let mut profiler =
        wgpu_profiler::GpuProfiler::new(context.device(), Default::default()).unwrap();
    let scope = profiler.scope("run", &mut encoder);
    let _output = pipeline_part
        .record(&mut context, &mut scope.into(), input, parameters)
        .unwrap();

    profiler.resolve_queries(&mut encoder);

    context.queue().submit([encoder.finish()]);

    profiler.end_frame().unwrap();

    tracing::info!("waiting");
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    profiler_output(&context, &mut profiler).unwrap();
}
