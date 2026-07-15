// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

#[test]
fn single() {
    let bits = vec![0b0001001010];
    let pops = vec![3];

    assert_eq!(
        pops,
        run(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
            },
            &bits,
        )
    );
}

#[test]
fn random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let bits: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .take(1000)
        .collect();
    let pops: Vec<u32> = bits.iter().map(|b| b.count_ones()).collect();

    assert_eq!(
        pops,
        run(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
            },
            &bits,
        )
    );
}

fn run(settings: Settings, bits: &[u32]) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings,
        (u16::MAX as u32).try_into().unwrap(),
        bits,
    )
    .unwrap();
    let bits_to_pops = BitsToPops::new(&mut context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { pops } = bits_to_pops
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, pops);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
