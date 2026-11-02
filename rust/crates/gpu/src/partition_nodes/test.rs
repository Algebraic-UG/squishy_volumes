// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use nalgebra::Vector3;

use super::*;

fn check(settings: Settings, positions_and_collider_bits: &[PositionAndColliderBits]) {
    let nodes = get_node_set(settings.grid_node_size, positions_and_collider_bits);
    let owns = run(settings, positions_and_collider_bits);

    for own in &owns {
        println!("{own:032b}");
    }

    let total = owns.into_iter().map(|own| own.count_ones()).sum::<u32>();
    assert_eq!(total as usize, nodes.len());
}

#[test]
fn test_single() {
    let positions = [PositionAndColliderBits {
        position: Vector3::zeros(),
        collider_bits: 0,
    }];

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 1.,
            table_tries: 50,
        },
        &positions,
    );
}

#[test]
fn test_simple() {
    let positions = [
        Vector3::new(-0.5, -0.5, -0.5),
        Vector3::new(-0.5, -0.5, 0.5),
        Vector3::new(-0.5, 0.5, -0.5),
        Vector3::new(-0.5, 0.5, 0.5),
        Vector3::new(0.5, -0.5, -0.5),
        Vector3::new(0.5, -0.5, 0.5),
        Vector3::new(0.5, 0.5, -0.5),
        Vector3::new(0.5, 0.5, 0.5),
    ];
    let positions_and_collider_bits = positions
        .into_iter()
        .map(|position| PositionAndColliderBits {
            position,
            collider_bits: 0,
        })
        .collect::<Vec<_>>();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 1.1,
            table_tries: 50,
        },
        &positions_and_collider_bits,
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let n = 1000;
    let positions: Vec<Vector3<f32>> = bytemuck::cast_vec(
        ChaCha8Rng::seed_from_u64(42)
            .random_iter::<f32>()
            .take(n * 3)
            .collect(),
    );
    let positions_and_collider_bits = positions
        .into_iter()
        .zip(ChaCha8Rng::seed_from_u64(42).random_iter::<u32>())
        .map(|(position, collider_bits)| PositionAndColliderBits {
            position,
            collider_bits,
        })
        .collect::<Vec<_>>();

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 1337.,
            table_tries: 50,
        },
        &positions_and_collider_bits,
    );
}

fn run(settings: Settings, positions_and_collider_bits: &[PositionAndColliderBits]) -> Vec<u32> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), positions_and_collider_bits).unwrap();
    let partition_nodes = PartitionNodes::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { owns } = partition_nodes
        .record(
            &mut context,
            &mut (&mut encoder).into(),
            input,
            Parameters {
                max_num_grid_nodes: (positions_and_collider_bits.len() as u32 * 27)
                    .try_into()
                    .unwrap(),
            },
        )
        .unwrap();

    let download = DownloadToHost::new(&context, owns);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
