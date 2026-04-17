// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

#[test]
fn test_simple() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = 4.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
    };

    let numbers = [0, 1, 1, 1, 1, 1, 0, 1, 1, 1];
    let prefixes = prefix_sum_on_cpu(&numbers);

    let indirect = Indirect::new(IndirectSettings {
        workgroup_size,
        dispatch_limit,
        len: prefixes.last().unwrap() + 1,
    });
    assert_eq!(vec![indirect], run_offsets_to_indirect(settings, &prefixes),);
}

#[test]
fn test_random() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = 4.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
    };

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<bool>()
        .map(|bit| if bit { 1 } else { 0 })
        .take(10000)
        .collect();
    let prefixes = prefix_sum_on_cpu(&numbers);

    println!("{}", numbers.last().unwrap());
    println!("{}", prefixes.last().unwrap());

    let indirect = Indirect::new(IndirectSettings {
        workgroup_size,
        dispatch_limit,
        len: prefixes.last().unwrap() + 1,
    });
    assert_eq!(vec![indirect], run_offsets_to_indirect(settings, &prefixes),);
}

fn run_offsets_to_indirect(settings: Settings, offsets: &[u32]) -> Vec<Indirect> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, offsets);
    let offsets_to_indirect = OffsetsToIndirect::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output { new_indirect } = offsets_to_indirect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, new_indirect);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
