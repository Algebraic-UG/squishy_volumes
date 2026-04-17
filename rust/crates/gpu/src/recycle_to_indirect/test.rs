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
    keys: &[u32],
) {
    let subgroup_size = get_subgroup_size();
    let indices: Vec<_> = (0..keys.len() as u32).collect();
    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        3,
        0,
        workgroup_size.get(),
        subgroup_size,
        &indices,
        keys,
    );
    let prefixes = prefix_sum_on_cpu(&counts);

    let counts = (0..8)
        .map(|colorkey| keys.iter().filter(|key| **key == colorkey).count() as u32)
        .collect::<Vec<_>>();
    // inclusive prefix sum here
    let limits: Vec<u32> = counts
        .iter()
        .scan(0, |prefix_sum, item| {
            *prefix_sum += item;
            Some(*prefix_sum)
        })
        .collect();
    let indirect_colors = counts
        .into_iter()
        .zip(limits)
        .map(|(len, limit)| {
            let mut indirect = Indirect::new(IndirectSettings {
                workgroup_size,
                dispatch_limit,
                len,
            });
            indirect.len = limit;
            indirect
        })
        .collect::<Vec<_>>();

    assert_eq!(
        indirect_colors,
        run_recycle_to_indirect(settings, keys.len() as u32, &prefixes),
    );
}

#[test]
fn test_simple() {
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        dispatch_limit: 4.try_into().unwrap(),
    };

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    check(settings, &keys);
}

#[test]
fn test_random() {
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
        dispatch_limit: 100.try_into().unwrap(),
    };

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .map(|key| key & 0b111)
        .take(10000)
        .collect();

    check(settings, &keys);
}

fn run_recycle_to_indirect(settings: Settings, len: u32, prefix_sums: &[u32]) -> Vec<Indirect> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, len, prefix_sums);
    let recycle_to_indirect = RecycleToIndirect::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output { indirect_colors } = recycle_to_indirect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, indirect_colors);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
