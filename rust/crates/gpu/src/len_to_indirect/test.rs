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
    let len = numbers.iter().sum();

    let indirect = Indirect::new(DispatchSettings {
        workgroup_size,
        dispatch_limit,
        len,
    });

    assert_eq!(vec![indirect], run(settings, len),);
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

fn run(settings: Settings, len: u32) -> Vec<Indirect> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), len).unwrap();
    let len_to_indirect = LenToIndirect::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output { new_indirect } = len_to_indirect
        .record(
            &mut context,
            &mut (&mut encoder).into(),
            input,
            Parameters { limit: len },
        )
        .unwrap();

    let download = DownloadToHost::new(&context, new_indirect);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
