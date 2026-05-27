// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_util::{Aabb, NORMALIZATION_EPS, rasterization::candidates};

use super::*;

fn check(
    settings @ Settings {
        cell_size, layers, ..
    }: Settings,
    input_data @ InputData {
        block_ids,
        block_table,
        ..
    }: InputData,
) {
    let mut collider_bits_cpu: Vec<u32> = vec![0; block_ids.len()];
    let table_mask = block_table.len() as u32 - 1;
    for (collider_index, (vertices, triangles)) in input_data.collider_meshes.iter().enumerate() {
        for triangle in *triangles {
            let a = vertices[triangle.a as usize].xyz();
            let b = vertices[triangle.b as usize].xyz();
            let c = vertices[triangle.c as usize].xyz();
            let ab = a - b;
            let bc = b - c;
            let ca = c - a;

            let normal_area_2 = (-ab).cross(&ca);
            let area_2 = normal_area_2.norm();
            if area_2 < NORMALIZATION_EPS {
                continue;
            }
            let n = normal_area_2 / area_2;

            'candidate_loop: for candidate in
                candidates(&a, &b, &c, &n, cell_size / 2., layers as usize)
            {
                let mut slot = cell_to_murmur(&candidate.push(0)) & table_mask;
                let block_index = loop {
                    let entry = block_table[slot as usize];
                    if entry == 0 {
                        continue 'candidate_loop;
                    }
                    let block_index = entry as usize - 1;
                    if block_ids[block_index].xyz() == candidate.xyz() {
                        break block_index;
                    }
                    slot += 1;
                    slot &= table_mask;
                };
                collider_bits_cpu[block_index] |= 1 << collider_index;
            }
        }
    }
    let collider_pops_cpu: Vec<u32> = collider_bits_cpu
        .iter()
        .map(|bits| bits.count_ones())
        .collect();

    let (collider_bits_gpu, collider_pops_gpu) = run(settings, input_data);

    assert_eq!(collider_bits_cpu, collider_bits_gpu);
    assert_eq!(collider_pops_cpu, collider_pops_gpu);
}

#[test]
fn single() {
    let vertices = vec![
        Vector4::new(1., 1., 1., 0.),
        Vector4::new(0., 1., 0., 0.),
        Vector4::new(1., 0., 0., 0.),
    ];
    let triangles = vec![Triangle { a: 0, b: 1, c: 2 }];

    let cell_ids: Vec<_> = (-10..=10)
        .flat_map(move |i| {
            (-10..=10).flat_map(move |j| (-10..=10).map(move |k| Vector4::new(i, j, k, 0)))
        })
        .collect();
    let (block_ids, block_table) = build_hash_table_on_cpu_simple(&cell_ids);

    check(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            cell_size: 0.5,
            layers: 3,
        },
        InputData {
            collider_meshes: vec![(&vertices, &triangles)],
            block_ids: &block_ids,
            block_table: &block_table,
        },
    );
}

fn run(settings: Settings, input_data: InputData) -> (Vec<u32>, Vec<u32>) {
    let mut context = SHARED_CONTEXT.lock().unwrap();
    let input = Input::new(context.device(), settings, input_data);
    let count_colliders = CountColliders::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output {
        collider_bits,
        collider_pops,
    } = count_colliders
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(&context, [collider_bits, collider_pops]);
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();
    let [collider_bits, collider_pops] = downloads.try_into().unwrap();

    (collider_bits.to_vec(), collider_pops.to_vec())
}
