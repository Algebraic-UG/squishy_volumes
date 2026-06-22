// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;
use nalgebra::{Matrix1x3, Matrix3, Vector3, stack};

fn check(settings: Settings, input_data: InputData) {
    let _ = run(settings, input_data);
    todo!();
}

#[test]
fn test_single_undeformed() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let grid_node_size = 1.;
    let time_step = 0.001;
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        grid_node_size,
        time_step,
    };

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
    let node_momentums = vec![Vector4::zeros(); 27];

    check(
        settings,
        InputData {
            node_ids_and_collider_bits: &node_ids_and_collider_bits,
            node_momentums: &node_momentums,
            particle_positions_and_collider_bits: &[PositionAndColliderBits {
                position: Vector3::repeat(0.6),
                collider_bits: 0,
            }],
            particle_position_gradients: &[stack![
                Matrix3::identity();
                Matrix1x3::zeros()
            ]],
            particle_velocities: &[Vector4::zeros()],
            particle_velocity_gradients: &[stack![
                Matrix3::zeros();
                Matrix1x3::zeros()
            ]],
        },
    );
}

fn run(
    settings: Settings,
    input_data: InputData,
) -> (
    Vec<PositionAndColliderBits>,
    Vec<Matrix4x3<f32>>,
    Vec<Vector4<f32>>,
    Vec<Matrix4x3<f32>>,
) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), input_data);
    let particle_positions_and_collider_bits = input.particle_positions_and_collider_bits.clone();
    let particle_position_gradients = input.particle_position_gradients.clone();
    let particle_velocities = input.particle_velocities.clone();
    let particle_velocity_gradients = input.particle_velocity_gradients.clone();

    let collect = Collect::new(&context, settings);

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = collect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            particle_positions_and_collider_bits,
            particle_position_gradients,
            particle_velocities,
            particle_velocity_gradients,
        ],
    );
    downloads.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);

    let downloads = downloads.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    let [
        particle_positions_and_collider_bits,
        particle_position_gradients,
        particle_velocities,
        particle_velocity_gradients,
    ] = downloads.try_into().unwrap();
    (
        particle_positions_and_collider_bits.to_vec(),
        particle_position_gradients.to_vec(),
        particle_velocities.to_vec(),
        particle_velocity_gradients.to_vec(),
    )
}
