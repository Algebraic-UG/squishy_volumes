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
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 0;
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();

    assert_eq!(
        count_subkeys_on_cpu(
            dispatch_limit.get(),
            bit_count.get(),
            bit_offset,
            workgroup_size.get(),
            subgroup_size.get(),
            &indices,
            &keys
        ),
        run_subkey_count(settings, parameters, None, &keys),
    );

    shuffle(&mut indices, 1);

    assert_eq!(
        count_subkeys_on_cpu(
            dispatch_limit.get(),
            bit_count.get(),
            bit_offset,
            workgroup_size.get(),
            subgroup_size.get(),
            &indices,
            &keys
        ),
        run_subkey_count(settings, parameters, Some(&indices), &keys),
    );
}

#[test]
fn test_simple_with_offset() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 2; // just this is different
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();

    assert_eq!(
        count_subkeys_on_cpu(
            dispatch_limit.get(),
            bit_count.get(),
            bit_offset,
            workgroup_size.get(),
            subgroup_size.get(),
            &indices,
            &keys
        ),
        run_subkey_count(settings, parameters, None, &keys),
    );

    shuffle(&mut indices, 2); // and a different seed, why not

    assert_eq!(
        count_subkeys_on_cpu(
            dispatch_limit.get(),
            bit_count.get(),
            bit_offset,
            workgroup_size.get(),
            subgroup_size.get(),
            &indices,
            &keys
        ),
        run_subkey_count(settings, parameters, Some(&indices), &keys),
    );
}

#[test]
fn test_larger() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 0;
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys = [1; 513];
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();
    assert_eq!(
        count_subkeys_on_cpu(
            dispatch_limit.get(),
            bit_count.get(),
            bit_offset,
            workgroup_size.get(),
            subgroup_size.get(),
            &indices,
            &keys
        ),
        run_subkey_count(settings, parameters, None, &keys),
    );

    shuffle(&mut indices, 3);

    assert_eq!(
        count_subkeys_on_cpu(
            dispatch_limit.get(),
            bit_count.get(),
            bit_offset,
            workgroup_size.get(),
            subgroup_size.get(),
            &indices,
            &keys
        ),
        run_subkey_count(settings, parameters, Some(&indices), &keys),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let subgroup_size = get_subgroup_size();

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let mut indices: Vec<u32> = (0..keys.len() as u32).collect();

    for bit_offset in 0..5 {
        let parameters = Parameters { bit_offset };

        assert_eq!(
            count_subkeys_on_cpu(
                dispatch_limit.get(),
                bit_count.get(),
                bit_offset,
                workgroup_size.get(),
                subgroup_size.get(),
                &indices,
                &keys
            ),
            run_subkey_count(settings, parameters, None, &keys),
        );
    }

    shuffle(&mut indices, 4);

    for bit_offset in 0..5 {
        let parameters = Parameters { bit_offset };

        assert_eq!(
            count_subkeys_on_cpu(
                dispatch_limit.get(),
                bit_count.get(),
                bit_offset,
                workgroup_size.get(),
                subgroup_size.get(),
                &indices,
                &keys
            ),
            run_subkey_count(settings, parameters, Some(&indices), &keys),
        );
    }
}

fn run_subkey_count(
    settings: Settings,
    parameters: Parameters,
    indices: Option<&[u32]>,
    keys: &[u32],
) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, indices, keys).unwrap();

    let count_subkeys = CountSubkeys::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { counts } = count_subkeys
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, counts);

    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
