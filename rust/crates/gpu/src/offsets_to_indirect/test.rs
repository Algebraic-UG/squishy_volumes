// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

fn check(
    settings @ Settings {
        workgroup_size,
        dispatch_limit,
    }: Settings,
    numbers: &[u32],
) {
    let subgroup_size = SHARED_CONTEXT.lock().unwrap().subgroup_size();
    let prefixes = prefix_sum_on_cpu(&numbers);

    let indirect = Indirect::new(IndirectSettings {
        workgroup_size,
        dispatch_limit,
        len: prefixes.last().unwrap() + 1,
    });
    let mut indirect_batch = Indirect::new(IndirectSettings {
        workgroup_size,
        dispatch_limit,
        len: (prefixes.last().unwrap() + 1) * subgroup_size.get(),
    });
    indirect_batch.len = indirect.len;

    assert_eq!(
        (vec![indirect], vec![indirect_batch]),
        run_offsets_to_indirect(settings, &prefixes),
    );
}

#[test]
fn test_simple() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = 4.try_into().unwrap();

    let numbers = [0, 1, 1, 1, 1, 1, 0, 1, 1, 1];
    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &numbers,
    );
}

#[test]
fn test_random() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = 4.try_into().unwrap();

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<bool>()
        .map(|bit| if bit { 1 } else { 0 })
        .take(10000)
        .collect();
    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &numbers,
    );
}

fn run_offsets_to_indirect(settings: Settings, offsets: &[u32]) -> (Vec<Indirect>, Vec<Indirect>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, offsets);
    let offsets_to_indirect = OffsetsToIndirect::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output {
        new_indirect,
        new_indirect_batch,
    } = offsets_to_indirect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [new_indirect, new_indirect_batch]);
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [new_indirect, new_indirect_batch] = downloads.try_into().unwrap();
    (new_indirect.to_vec(), new_indirect_batch.to_vec())
}
