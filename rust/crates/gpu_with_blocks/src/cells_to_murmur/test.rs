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
    let cells = [
        Vector4::new(-5, -5, -5, 0),
        Vector4::new(-5, -5, 5, 0),
        Vector4::new(-5, 5, -5, 0),
        Vector4::new(-5, 5, 5, 0),
        Vector4::new(5, -5, -5, 0),
        Vector4::new(5, -5, 5, 0),
        Vector4::new(5, 5, -5, 0),
        Vector4::new(5, 5, 5, 0),
    ];

    assert_eq!(
        cells_to_murmur_on_cpu(&cells),
        run_cells_to_murmur(
            Settings {
                workgroup_size: 64.try_into().unwrap()
            },
            (u16::MAX as u32).try_into().unwrap(),
            &cells
        ),
    );
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

    assert_eq!(
        cells_to_murmur_on_cpu(&cells),
        run_cells_to_murmur(
            Settings {
                workgroup_size: 64.try_into().unwrap()
            },
            (u16::MAX as u32).try_into().unwrap(),
            &cells
        ),
    );
}

fn run_cells_to_murmur(
    settings: Settings,
    dispatch_limit: NonZeroU32,
    cells: &[Vector4<i32>],
) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        cells,
    );

    let cells_to_murmur = CellsToMurmur::new(&context, settings);
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { hashes } = cells_to_murmur
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, hashes);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
