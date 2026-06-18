// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;

use super::*;

#[test]
fn test_simple() {
    let node_ids = [
        Vector3::new(-5, -5, -5),
        Vector3::new(-5, -5, 5),
        Vector3::new(-5, 5, -5),
        Vector3::new(-5, 5, 5),
        Vector3::new(5, -5, -5),
        Vector3::new(5, -5, 5),
        Vector3::new(5, 5, -5),
        Vector3::new(5, 5, 5),
    ];

    assert_eq!(
        node_ids.iter().map(node_id_to_murmur).collect::<Vec<_>>(),
        run(
            Settings {
                workgroup_size: 64.try_into().unwrap()
            },
            (u16::MAX as u32).try_into().unwrap(),
            &node_ids
        ),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let node_ids: Vec<i32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<i32>()
        .take(1000 * 3)
        .collect();
    let node_ids: Vec<Vector3<i32>> = node_ids
        .chunks_exact(3)
        .map(Vector3::from_column_slice)
        .collect();

    assert_eq!(
        node_ids.iter().map(node_id_to_murmur).collect::<Vec<_>>(),
        run(
            Settings {
                workgroup_size: 64.try_into().unwrap()
            },
            (u16::MAX as u32).try_into().unwrap(),
            &node_ids
        ),
    );
}

fn run(settings: Settings, dispatch_limit: NonZeroU32, node_ids: &[Vector3<i32>]) -> Vec<u32> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(
        context.device(),
        settings.workgroup_size,
        dispatch_limit,
        node_ids,
    );

    let node_ids_to_murmur = NodeIdsToMurmur::new(&context, settings);
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { hashes } = node_ids_to_murmur
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
