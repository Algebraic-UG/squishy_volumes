use std::{
    fs::{File, read},
    io::Write,
    path::PathBuf,
};

use squishy_volumes_gpu::{
    CountSubkeysSettings, PrefixSumSettings, RadixSortSettings, ReorderSettings, prefix_sum_on_cpu,
    shuffle, sort_on_cpu,
};
use tracing::{dispatcher::set_global_default, info};
use tracing_subscriber::FmtSubscriber;

use clap::{Parser, ValueEnum};

use crate::{prefix_sum::prefix_sum_on_gpu, radix_sort::radix_sort_on_gpu};

mod prefix_sum;
mod radix_sort;
mod window;

#[derive(Debug, ValueEnum, Clone)]
enum Mode {
    Cpu,
    Gpu,
}

#[derive(ValueEnum, Clone, Copy, Default)]
enum Tool {
    #[default]
    RenderDoc,
    Nsight,
}

#[derive(Debug, ValueEnum, Clone)]
enum Task {
    Sum,
    Sort,
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
        input_file.unwrap_or(test_data.join(format!("{task:?}-in.bin").to_lowercase()));
    let output_file =
        output_file.unwrap_or(test_data.join(format!("{mode:?}-{task:?}-out.bin").to_lowercase()));

    if let Some(generate) = generate {
        let mut input: Vec<_> = (0..generate).collect();
        shuffle(&mut input, 24);

        let mut out = File::create(&input_file).unwrap();
        out.write_all(bytemuck::cast_slice(&input)).unwrap();

        info!("Generated {} numbers", input.len());
    };

    let input_bytes = read(input_file).unwrap();
    let input: &[u32] = bytemuck::cast_slice(&input_bytes);

    info!("Parsed {} numbers", input.len());

    let mut indices: Vec<u32> = (0..input.len() as u32).collect();
    shuffle(&mut indices, 42);

    let output = match (mode, task) {
        (Mode::Cpu, Task::Sum) => prefix_sum_on_cpu(input),
        (Mode::Cpu, Task::Sort) => sort_on_cpu(&indices, input),
        (Mode::Gpu, Task::Sum) => {
            prefix_sum_on_gpu(tool, PrefixSumSettings { workgroup_size }, input)
        }
        (Mode::Gpu, Task::Sort) => radix_sort_on_gpu(
            tool,
            RadixSortSettings {
                count_subkeys_settings: CountSubkeysSettings {
                    workgroup_size,
                    bit_count,
                },
                prefix_sum_settings: PrefixSumSettings { workgroup_size },
                reorder_settings: ReorderSettings {
                    workgroup_size,
                    bit_count,
                },
            },
            &indices,
            input,
        ),
    };

    let mut out = File::create(output_file).unwrap();
    out.write_all(bytemuck::cast_slice(&output)).unwrap();
}
