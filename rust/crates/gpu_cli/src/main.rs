use std::{
    fs::{File, read},
    io::Write,
    path::PathBuf,
};

use convert_case::{Case, Casing};
use nalgebra::Vector4;
use rand::{random_iter, rng, seq::SliceRandom};
use squishy_volumes_gpu::{
    CountSubkeysSettings, PositionsToKeysParameters, PositionsToKeysSettings, PrefixSumSettings,
    RadixSortSettings, ReorderSettings, SortPositionsIntoCellsSettings, positions_to_keys_on_cpu,
    prefix_sum_on_cpu, shuffle, sort_on_cpu, sort_positions_into_cells_on_cpu,
};
use tracing::{dispatcher::set_global_default, info};
use tracing_subscriber::FmtSubscriber;

use clap::{Parser, ValueEnum};

use crate::{
    positions_to_keys::positions_to_keys_on_gpu, prefix_sum::prefix_sum_on_gpu,
    radix_sort::radix_sort_on_gpu, sort_positions_into_cells::sort_positions_into_cells_on_gpu,
};

mod positions_to_keys;
mod prefix_sum;
mod radix_sort;
mod sort_positions_into_cells;
mod window;

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

    #[arg(long, default_value_t = 64)]
    workgroup_size: u32,

    #[arg(long, default_value_t = 2)]
    bit_count: u32,
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
        bit_count,
    } = Cli::parse();

    let test_data = PathBuf::from("test_data");
    let input_file =
        input_file.unwrap_or(test_data.join(format!("{task:?}-in.bin").to_case(Case::Kebab)));
    let output_file = output_file
        .unwrap_or(test_data.join(format!("{mode:?}-{task:?}-out.bin").to_case(Case::Kebab)));

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
        }

        info!("Generation done.");
    };

    let input_bytes = read(input_file).unwrap();

    let cell_size = 1337.;
    let prefix_sum_settings = PrefixSumSettings { workgroup_size };
    let radix_sort_settings = RadixSortSettings {
        count_subkeys_settings: CountSubkeysSettings {
            workgroup_size,
            bit_count,
        },
        prefix_sum_settings,
        reorder_settings: ReorderSettings {
            workgroup_size,
            bit_count,
        },
    };
    let positions_to_keys_settings = PositionsToKeysSettings {
        workgroup_size,
        cell_size,
    };
    let sort_positions_into_cells_settings = SortPositionsIntoCellsSettings {
        positions_to_keys_settings,
        radix_sort_settings,
    };

    let mut out = File::create(output_file).unwrap();
    match task {
        Task::Sum | Task::Sort => {
            let input: &[u32] = bytemuck::cast_slice(&input_bytes);
            let output = match task {
                Task::Sum => match mode {
                    Mode::Cpu => prefix_sum_on_cpu(input),
                    Mode::Gpu => prefix_sum_on_gpu(tool, prefix_sum_settings, input),
                },
                Task::Sort => {
                    let mut indices: Vec<u32> = (0..input.len() as u32).collect();
                    shuffle(&mut indices, 42);

                    match mode {
                        Mode::Cpu => sort_on_cpu(&indices, input),
                        Mode::Gpu => radix_sort_on_gpu(tool, radix_sort_settings, &indices, input),
                    }
                }
                _ => unreachable!(),
            };
            out.write_all(bytemuck::cast_slice(&output)).unwrap();
        }
        Task::PositionsToKeys | Task::SortIntoCells => {
            let input: &[Vector4<f32>] = bytemuck::cast_slice(&input_bytes);
            let mut indices: Vec<u32> = (0..input.len() as u32).collect();
            shuffle(&mut indices, 42);

            let output = match task {
                Task::PositionsToKeys => {
                    let dimension = 1;
                    match mode {
                        Mode::Cpu => positions_to_keys_on_cpu(input, cell_size, dimension),
                        Mode::Gpu => positions_to_keys_on_gpu(
                            tool,
                            positions_to_keys_settings,
                            PositionsToKeysParameters { dimension },
                            input,
                        ),
                    }
                }
                Task::SortIntoCells => match mode {
                    Mode::Cpu => sort_positions_into_cells_on_cpu(&indices, input, cell_size),
                    Mode::Gpu => sort_positions_into_cells_on_gpu(
                        tool,
                        sort_positions_into_cells_settings,
                        &indices,
                        input,
                    ),
                },
                _ => unreachable!(),
            };
            out.write_all(bytemuck::cast_slice(&output)).unwrap();
        }
    }
}
