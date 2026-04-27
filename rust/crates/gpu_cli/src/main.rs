use std::{
    fs::{File, read},
    io::Write,
    num::NonZeroU32,
    path::PathBuf,
};

use convert_case::{Case, Casing};
use nalgebra::Vector4;
use rand::{random_iter, rng, seq::SliceRandom};
use squishy_volumes_gpu::{
    grid_on_cpu, i32_to_u32_offset, positions_to_keys_on_cpu, prefix_sum_on_cpu, shuffle,
    sort_on_cpu, sort_positions_into_cells_on_cpu,
};
use tracing::{dispatcher::set_global_default, info};
use tracing_subscriber::FmtSubscriber;

use clap::{Parser, ValueEnum};

use squishy_volumes_gpu as gpu;

use crate::{
    build_hash_table::build_hash_table_on_gpu, positions_to_keys::positions_to_keys_on_gpu,
    prefix_sum::prefix_sum_on_gpu, prepare_grid::prepare_grid_on_gpu,
    radix_sort::radix_sort_on_gpu, scatter::scatter_on_gpu,
    sort_positions_into_cells::sort_positions_into_cells_on_gpu,
};

mod build_hash_table;
mod positions_to_keys;
mod prefix_sum;
mod prepare_grid;
mod radix_sort;
mod scatter;
mod sort_positions_into_cells;
mod window;

mod profiler_output;

#[derive(Debug, ValueEnum, Clone)]
enum Mode {
    Cpu,
    Gpu,
}

#[derive(ValueEnum, Clone, Copy, Default, Debug)]
enum Tool {
    #[default]
    RenderDoc,
    Nsight,
}

#[derive(Debug, ValueEnum, Clone)]
enum Task {
    Sum,
    Sort,
    PositionsToKeys,
    SortIntoCells,
    BuildHashTable,
    PrepareGrid,
    Scatter,
}

#[derive(Parser)]
struct Cli {
    #[arg(value_enum)]
    mode: Mode,

    #[arg(value_enum)]
    task: Task,

    #[arg(
        long,
        value_name = "input file containing the numbers, defaults to test_data/<task>-in.bin"
    )]
    input_file: Option<PathBuf>,

    #[arg(
        long,
        value_name = "output file for the prefix sums, defaults to test_data/<mode>-<task>-out.bin"
    )]
    output_file: Option<PathBuf>,

    #[arg(
        long,
        value_name = "generate given amount of input and overwrite input file"
    )]
    generate: Option<u32>,

    #[arg(long, value_enum)]
    tool: Option<Tool>,

    #[arg(long, default_value_t = NonZeroU32::new(64).unwrap())]
    workgroup_size: NonZeroU32,

    #[arg(long, default_value_t = NonZeroU32::new(u16::MAX as u32).unwrap())]
    dispatch_limit: NonZeroU32,

    #[arg(long, default_value_t = NonZeroU32::new(2).unwrap())]
    bit_count: NonZeroU32,
}

fn main() {
    set_global_default(FmtSubscriber::default().into()).unwrap();

    let Cli {
        mode,
        task,
        input_file,
        output_file,
        generate,
        tool,
        workgroup_size,
        dispatch_limit,
        bit_count,
    } = Cli::parse();

    let test_data = PathBuf::from("test_data");
    let input_file =
        input_file.unwrap_or(test_data.join(format!("{task:?}-in.bin").to_case(Case::Kebab)));
    let output_file = output_file
        .unwrap_or(test_data.join(format!("{mode:?}-{task:?}-out.bin").to_case(Case::Kebab)));

    let cell_size = 1.;
    if let Some(generate) = generate {
        let mut out = File::create(&input_file).unwrap();
        match task {
            Task::Sum => {
                // trying not to overflow
                let mut input: Vec<_> = (0..generate).collect();
                input.shuffle(&mut rng());
                out.write_all(bytemuck::cast_slice(&input)).unwrap();
            }
            Task::Sort => {
                // for sorting we can go arbitrary large
                let keys: Vec<u32> = random_iter().take(generate as usize).collect();
                out.write_all(bytemuck::cast_slice(&keys)).unwrap();
            }
            Task::PositionsToKeys | Task::SortIntoCells => {
                let positions: Vec<Vector4<f32>> = (0..generate)
                    .map(|_| Vector4::new_random())
                    .take(generate as usize)
                    .collect();
                out.write_all(bytemuck::cast_slice(&positions)).unwrap();
            }
            Task::Scatter => {
                let per_dim = (generate as f64).powf(1. / 3.).ceil() as usize;
                let input: Vec<_> = (0..per_dim)
                    .flat_map(move |x| {
                        (0..per_dim).flat_map(move |y| {
                            (0..per_dim).map(move |z| {
                                Vector4::new(x as f32, y as f32, z as f32, 0.)
                                    .scale(1. / cell_size / 4.)
                            })
                        })
                    })
                    .collect();
                assert!(input.len() >= generate as usize);
                out.write_all(bytemuck::cast_slice(&input)).unwrap();
            }
            Task::BuildHashTable => {
                let cells: Vec<Vector4<i32>> = (0..generate)
                    .map(|_| Vector4::new_random())
                    .take(generate as usize)
                    .collect();
                out.write_all(bytemuck::cast_slice(&cells)).unwrap();
            }
            Task::PrepareGrid => {
                let positions: Vec<Vector4<f32>> = (0..generate)
                    .map(|_| Vector4::new_random())
                    .take(generate as usize)
                    .collect();
                out.write_all(bytemuck::cast_slice(&positions)).unwrap();
            }
        }

        info!("Generation done.");
    };

    let input_bytes = read(input_file).unwrap();

    let prefix_sum = gpu::prefix_sum::Settings {
        workgroup_size,
        dispatch_limit,
    };
    let radix_sort = gpu::radix_sort::Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let mut out = File::create(output_file).unwrap();
    match task {
        Task::Sum | Task::Sort => {
            let input: &[u32] = bytemuck::cast_slice(&input_bytes);
            let output = match task {
                Task::Sum => match mode {
                    Mode::Cpu => prefix_sum_on_cpu(input),
                    Mode::Gpu => prefix_sum_on_gpu(tool, prefix_sum, input),
                },
                Task::Sort => {
                    let mut indices: Vec<u32> = (0..input.len() as u32).collect();
                    shuffle(&mut indices, 42);

                    match mode {
                        Mode::Cpu => sort_on_cpu(&indices, input),
                        Mode::Gpu => radix_sort_on_gpu(tool, radix_sort, &indices, input),
                    }
                }
                _ => unreachable!(),
            };
            out.write_all(bytemuck::cast_slice(&output)).unwrap();
        }
        Task::PositionsToKeys | Task::SortIntoCells => {
            let input: &[Vector4<f32>] = bytemuck::cast_slice(&input_bytes);

            let output = match task {
                Task::PositionsToKeys => {
                    let dimension = 1;
                    match mode {
                        Mode::Cpu => positions_to_keys_on_cpu(input, cell_size, dimension),
                        Mode::Gpu => positions_to_keys_on_gpu(
                            tool,
                            gpu::positions_to_keys::Settings {
                                workgroup_size,
                                cell_size,
                            },
                            gpu::positions_to_keys::Parameters { dimension },
                            input,
                        ),
                    }
                }
                Task::SortIntoCells => {
                    let mut indices: Vec<u32> = (0..input.len() as u32).collect();
                    shuffle(&mut indices, 42);

                    match mode {
                        Mode::Cpu => sort_positions_into_cells_on_cpu(&indices, input, cell_size),
                        Mode::Gpu => sort_positions_into_cells_on_gpu(
                            tool,
                            gpu::sort_positions_into_cells::Settings {
                                workgroup_size,
                                dispatch_limit,
                                cell_size,
                                bit_count,
                            },
                            input,
                        ),
                    }
                }
                _ => unreachable!(),
            };
            out.write_all(bytemuck::cast_slice(&output)).unwrap();
        }
        Task::Scatter => {
            let input: &[Vector4<f32>] = bytemuck::cast_slice(&input_bytes);
            let _output = match mode {
                Mode::Cpu => todo!(),
                Mode::Gpu => scatter_on_gpu(
                    tool,
                    gpu::scatter::Settings {
                        workgroup_size,
                        cell_size,
                    },
                    input,
                ),
            };
        }
        Task::BuildHashTable => {
            let input: &[Vector4<i32>] = bytemuck::cast_slice(&input_bytes);
            let output = match mode {
                Mode::Cpu => todo!(),
                Mode::Gpu => build_hash_table_on_gpu(
                    tool,
                    gpu::build_hash_table::Settings { workgroup_size },
                    input,
                ),
            };
            out.write_all(bytemuck::cast_slice(&output)).unwrap();
        }
        Task::PrepareGrid => {
            let input: &[Vector4<f32>] = bytemuck::cast_slice(&input_bytes);
            let indices: Vec<u32> = (0..input.len() as u32).collect();
            let mut output = match mode {
                Mode::Cpu => grid_on_cpu(cell_size, &indices, input),
                Mode::Gpu => prepare_grid_on_gpu(
                    tool,
                    gpu::prepare_grid::Settings {
                        workgroup_size,
                        dispatch_limit,
                        bit_count,
                        cell_size,
                    },
                    input,
                ),
            };
            output.sort_by(|a, b| {
                a.map(i32_to_u32_offset)
                    .iter()
                    .cmp(b.map(i32_to_u32_offset).iter())
            });
            out.write_all(bytemuck::cast_slice(&output)).unwrap();
        }
    }
}
