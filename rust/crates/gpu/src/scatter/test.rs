// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::Vector3;

use super::*;

fn check(settings: Settings, dispatch_limit: NonZeroU32, input_data: InputData) {
    let gpu_node_momentums = run(settings, dispatch_limit, input_data);
    println!("{gpu_node_momentums:?}");
    todo!();

    /*
    let grid_cpu = scatter_on_cpu(cell_size, time_step, input_data.clone());

    println!("{:?}", grid_cpu);
    println!("{:?}", grid_cpu.values().collect::<Vec<_>>());

    let (blocks, block_ids) = run_scatter(settings, dispatch_limit, input_data);

    for (block_index, (block, block_id)) in blocks.iter().zip(&block_ids).enumerate() {
        println!("block {block_index}, {block_id:?}");
        let low_node = block_id * 2 - Vector4::repeat(1);
        for node in 0..8 {
            let node_id = low_node + block_offset(node as u32);
            if let Some(cpu) = grid_cpu.get(&node_id.xyz()) {
                println!("both have {:?}", node_id.xyz());
                check_iters(cpu.iter(), block.nodes[node].iter());
            } else {
                assert_eq!(block.nodes[node], Vector4::zeros());
            }
        }
    }

    let super_set: HashSet<_> = gpu_grid_to_cpu_grid(&block_ids).into_iter().collect();
    for node in grid_cpu.keys() {
        assert!(super_set.contains(&node.push(0)));
    }
    */
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let settings = Settings {
        workgroup_size,
        grid_node_size,
    };

    let contributor_offsets = (0..27).collect::<Vec<_>>();
    let contributors = [0; 27];
    let node_ids = [
        Vector3::new(0, 0, 0),
        Vector3::new(0, 0, 1),
        Vector3::new(0, 0, 2),
        Vector3::new(0, 1, 0),
        Vector3::new(0, 1, 1),
        Vector3::new(0, 1, 2),
        Vector3::new(0, 2, 0),
        Vector3::new(0, 2, 1),
        Vector3::new(0, 2, 2),
        Vector3::new(1, 0, 0),
        Vector3::new(1, 0, 1),
        Vector3::new(1, 0, 2),
        Vector3::new(1, 1, 0),
        Vector3::new(1, 1, 1),
        Vector3::new(1, 1, 2),
        Vector3::new(1, 2, 0),
        Vector3::new(1, 2, 1),
        Vector3::new(1, 2, 2),
        Vector3::new(2, 0, 0),
        Vector3::new(2, 0, 1),
        Vector3::new(2, 0, 2),
        Vector3::new(2, 1, 0),
        Vector3::new(2, 1, 1),
        Vector3::new(2, 1, 2),
        Vector3::new(2, 2, 0),
        Vector3::new(2, 2, 1),
        Vector3::new(2, 2, 2),
    ];
    let node_ids_and_collider_bits = node_ids
        .into_iter()
        .map(|node_id| NodeIdAndColliderBits {
            node_id,
            collider_bits: 0,
        })
        .collect::<Vec<_>>();
    let particle_tmp = [Matrix4::new(
        0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 0., 1., 1., 1., 1.,
    )];

    check(
        settings,
        dispatch_limit,
        InputData {
            contributor_offsets: &contributor_offsets,
            contributors: &contributors,
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            particle_tmp: &particle_tmp,
        },
    );
}

fn run(settings: Settings, dispatch_limit: NonZeroU32, input_data: InputData) -> Vec<Vector4<f32>> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), settings, dispatch_limit, input_data);
    let scatter = Scatter::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { node_momentums } = scatter
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, node_momentums);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec()
}
