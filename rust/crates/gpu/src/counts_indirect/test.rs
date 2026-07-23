// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

fn check(len: u32) {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 3.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };

    let count_len = counts_count(CountsCountArgs {
        workgroup_size: workgroup_size.get(),
        subgroup_size: get_subgroup_size().get(),
        dispatch_limit: dispatch_limit.get(),
        counter: 2u32.pow(bit_count.get()),
        len,
    });

    let indirect = Indirect::new(DispatchSettings {
        workgroup_size,
        dispatch_limit,
        len: count_len,
    });
    assert_eq!(vec![indirect], run_counts_indirect(settings, len),);
}

#[test]
fn test_simple() {
    check(10)
}

#[test]
fn test_random() {
    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .take(100)
        .collect();
    for number in numbers {
        check(number);
    }
}

fn run_counts_indirect(settings: Settings, len: u32) -> Vec<Indirect> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, len).unwrap();
    let counts_indirect = CountsIndirect::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output { indirect_counts } = counts_indirect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, indirect_counts);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
