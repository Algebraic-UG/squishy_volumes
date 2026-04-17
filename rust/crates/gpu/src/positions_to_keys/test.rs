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
    let positions = [
        Vector4::new(-0.5, -0.5, -0.5, 0.),
        Vector4::new(-0.5, -0.5, 0.5, 0.),
        Vector4::new(-0.5, 0.5, -0.5, 0.),
        Vector4::new(-0.5, 0.5, 0.5, 0.),
        Vector4::new(0.5, -0.5, -0.5, 0.),
        Vector4::new(0.5, -0.5, 0.5, 0.),
        Vector4::new(0.5, 0.5, -0.5, 0.),
        Vector4::new(0.5, 0.5, 0.5, 0.),
    ];

    let keys_x: [i32; 8] = [-1, -1, -1, -1, 0, 0, 0, 0];
    let keys_y: [i32; 8] = [-1, -1, 0, 0, -1, -1, 0, 0];
    let keys_z: [i32; 8] = [-1, 0, -1, 0, -1, 0, -1, 0];

    let cell_size = 1.;

    assert_eq!(
        positions_to_keys_on_cpu(&positions, cell_size, 0)
            .into_iter()
            .map(u32_to_i32_offset)
            .collect::<Vec<_>>(),
        keys_x
    );
    assert_eq!(
        positions_to_keys_on_cpu(&positions, cell_size, 1)
            .into_iter()
            .map(u32_to_i32_offset)
            .collect::<Vec<_>>(),
        keys_y
    );
    assert_eq!(
        positions_to_keys_on_cpu(&positions, cell_size, 2)
            .into_iter()
            .map(u32_to_i32_offset)
            .collect::<Vec<_>>(),
        keys_z
    );

    let keys_x: Vec<_> = keys_x.into_iter().map(i32_to_u32_offset).collect();
    let keys_y: Vec<_> = keys_y.into_iter().map(i32_to_u32_offset).collect();
    let keys_z: Vec<_> = keys_z.into_iter().map(i32_to_u32_offset).collect();

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        cell_size,
    };

    assert_eq!(
        keys_x,
        run_positions_to_keys(
            settings,
            dispatch_limit,
            Parameters { dimension: 0 },
            &positions
        )
    );
    assert_eq!(
        keys_y,
        run_positions_to_keys(
            settings,
            dispatch_limit,
            Parameters { dimension: 1 },
            &positions
        )
    );
    assert_eq!(
        keys_z,
        run_positions_to_keys(
            settings,
            dispatch_limit,
            Parameters { dimension: 2 },
            &positions
        )
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cell_size = 1337.;
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        cell_size,
    };

    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(1000 * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();

    for dimension in [0, 1, 2] {
        assert_eq!(
            positions_to_keys_on_cpu(&positions, cell_size, dimension),
            run_positions_to_keys(
                settings,
                dispatch_limit,
                Parameters { dimension },
                &positions,
            ),
        );
    }
}

fn run_positions_to_keys(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    parameters: Parameters,
    positions: &[Vector4<f32>],
) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        positions,
    );
    let positions_to_keys = PositionsToKeys::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { keys } = positions_to_keys
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, keys);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
