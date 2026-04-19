// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(cells: &[Vector4<i32>]) {
    let settings = Settings {
        workgroup_size: 64.try_into().unwrap(),
    };
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    assert_eq!(
        cells_to_colorkeys_on_cpu(cells),
        run_cells_to_colorkeys(settings, dispatch_limit, cells),
    )
}

#[test]
fn test_simple() {
    let cells = [
        Vector4::new(-1, -1, 0, 0),
        Vector4::new(-1, 0, 0, 0),
        Vector4::new(-1, 1, 0, 0),
        Vector4::new(0, -1, 0, 0),
        Vector4::new(0, 0, 0, 0),
        Vector4::new(0, 1, 0, 0),
        Vector4::new(1, -1, 0, 0),
        Vector4::new(1, 0, 0, 0),
        Vector4::new(1, 1, 0, 0),
    ];

    check(&cells);
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let cells: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(1000 * 4)
        .collect();
    let cells: Vec<Vector4<i32>> = cells
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .collect();
    check(&cells);
}

fn run_cells_to_colorkeys(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    cell_ids: &[Vector4<i32>],
) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        cell_ids,
    );
    let cells_to_colorkeys = CellsToColorkeys::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { keys } = cells_to_colorkeys
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
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
