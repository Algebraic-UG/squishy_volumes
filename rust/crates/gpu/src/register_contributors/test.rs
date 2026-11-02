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

use std::iter::once;

use nalgebra::Vector3;
use rustc_hash::FxHashSet;

use super::*;

fn check(settings: Settings, positions_and_collider_bits: &[PositionAndColliderBits]) {
    let (node_ids_and_collider_bits, cpu_contributor_offsets, cpu_contributors) =
        contributors_on_cpu(settings.grid_node_size, positions_and_collider_bits);

    let (gpu_contributor_offsets, gpu_contributors) = run(
        settings,
        &node_ids_and_collider_bits,
        positions_and_collider_bits,
    );

    assert_eq!(cpu_contributor_offsets, gpu_contributor_offsets, "offsets");
    assert_eq!(
        cpu_contributors.len(),
        gpu_contributors.len(),
        "contributor lengths"
    );

    for (node_id_and_collider_bits, (start, end)) in node_ids_and_collider_bits.iter().zip(
        cpu_contributor_offsets.iter().cloned().zip(
            cpu_contributor_offsets
                .iter()
                .skip(1)
                .cloned()
                .chain(once(cpu_contributors.len() as u32)),
        ),
    ) {
        let cpu: FxHashSet<u32> = cpu_contributors.as_slice()[start as usize..end as usize]
            .iter()
            .cloned()
            .collect();
        assert_eq!(cpu.len() as u32, end - start);
        let gpu: FxHashSet<u32> = gpu_contributors.as_slice()[start as usize..end as usize]
            .iter()
            .cloned()
            .collect();
        assert_eq!(gpu.len() as u32, end - start);

        assert_eq!(
            cpu, gpu,
            "actual contributors for {node_id_and_collider_bits:?}"
        );
    }
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
            table_tries: 200, // TODO: why so many?
        },
        &positions_and_collider_bits,
    );
}

#[test]
fn specific() {
    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            grid_node_size: 0.5,
            table_tries: 50,
        },
        &specific_positions_and_collider_bits(),
    );
}

fn run(
    settings: Settings,
    node_ids_and_collider_bits: &[NodeIdAndColliderBits],
    positions_and_collider_bits: &[PositionAndColliderBits],
) -> (Vec<u32>, Vec<u32>) {
    let mut context = get_shared_context();

    let input = Input::new(
        context.device(),
        settings.clone(),
        node_ids_and_collider_bits,
        &vec![Default::default(); positions_and_collider_bits.len()],
        positions_and_collider_bits,
    )
    .unwrap();
    let register_contributors = RegisterContributors::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        contributor_offsets,
        contributors,
        ..
    } = register_contributors
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [contributor_offsets, contributors, context.status()],
    );
    downloads.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [contributor_offsets, contributors, status] = downloads.try_into().unwrap();

    status.to_vec::<GpuStatus>().unwrap()[0]
        .to_result(&context)
        .unwrap();

    (
        contributor_offsets.to_vec().unwrap(),
        contributors.to_vec().unwrap(),
    )
}
