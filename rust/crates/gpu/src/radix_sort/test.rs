// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

#[test]
fn test_simple() {
    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_radix_sort(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
                bit_count: 2.try_into().unwrap(),
            },
            &indices,
            &keys,
        )
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_radix_sort(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
                bit_count: 2.try_into().unwrap(),
            },
            &indices,
            &keys,
        )
    );
}

fn run_radix_sort(settings: Settings, indices: &[u32], keys: &[u32]) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings.clone(), indices, keys);

    let radix_sort = RadixSort::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { indices_out } = radix_sort
        .record_all_rounds(&mut context, &mut (&mut encoder).into(), input)
        .unwrap();

    let download = DownloadToHost::new(&context, indices_out);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);
    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
