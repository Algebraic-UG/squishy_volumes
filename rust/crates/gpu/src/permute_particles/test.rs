// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use nalgebra::{Matrix1x3, stack};

use super::*;

fn check(
    input_data @ InputData {
        permutation,
        indices,
        masses,
        initial_volumes,
        parameters,
        positions,
        position_gradients,
        velocities,
        velocity_gradients,
    }: InputData,
) {
    let indices_cpu = permutation.permute(indices);
    let masses_cpu = permutation.permute(masses);
    let initial_volumes_cpu = permutation.permute(initial_volumes);
    let parameters_cpu = permutation.permute(parameters);
    let positions_cpu = permutation.permute(positions);
    let position_gradients_cpu = permutation.permute(position_gradients);
    let velocities_cpu = permutation.permute(velocities);
    let velocity_gradients_cpu = permutation.permute(velocity_gradients);

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();

    let (
        indices_gpu,
        masses_gpu,
        initial_volumes_gpu,
        parameters_gpu,
        positions_gpu,
        position_gradients_gpu,
        velocities_gpu,
        velocity_gradients_gpu,
    ) = run_permute_particles(workgroup_size, dispatch_limit, input_data);

    println!("indices:");
    for (cpu, gpu) in indices_cpu.into_iter().zip(indices_gpu) {
        assert_eq!(cpu, gpu);
    }
    println!("masses:");
    for (cpu, gpu) in masses_cpu.into_iter().zip(masses_gpu) {
        assert_eq!(cpu, gpu);
    }
    println!("initial_volumes:");
    for (cpu, gpu) in initial_volumes_cpu.into_iter().zip(initial_volumes_gpu) {
        assert_eq!(cpu, gpu);
    }
    println!("parameters:");
    for (cpu, gpu) in parameters_cpu.into_iter().zip(parameters_gpu) {
        assert_eq!(cpu, gpu);
    }
    println!("positions:");
    for (cpu, gpu) in positions_cpu.into_iter().zip(positions_gpu) {
        check_iters(cpu.xyz().iter(), gpu.xyz().iter());
    }
    println!("position gradients:");
    for (cpu, gpu) in position_gradients_cpu
        .into_iter()
        .zip(position_gradients_gpu)
    {
        check_iters(
            cpu.fixed_view::<3, 3>(0, 0).iter(),
            gpu.fixed_view::<3, 3>(0, 0).iter(),
        );
    }
    println!("velocities:");
    for (cpu, gpu) in velocities_cpu.into_iter().zip(velocities_gpu) {
        check_iters(cpu.xyz().iter(), gpu.xyz().iter());
    }
    println!("velocity gradients:");
    for (cpu, gpu) in velocity_gradients_cpu
        .into_iter()
        .zip(velocity_gradients_gpu)
    {
        check_iters(
            cpu.fixed_view::<3, 3>(0, 0).iter(),
            gpu.fixed_view::<3, 3>(0, 0).iter(),
        );
    }
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let n = 1000;

    let mut rng = ChaCha8Rng::seed_from_u64(42);

    let mut permutation: Vec<_> = (0..n as u32).collect();
    shuffle(&mut permutation, 0);

    let indices: Vec<_> = (0..n as u32).collect();
    let masses = (0..n)
        .map(|_| rng.random_range(0.01..0.05))
        .collect::<Vec<_>>();
    let initial_volumes = (0..n)
        .map(|_| rng.random_range(0.01..0.05))
        .collect::<Vec<_>>();
    let particle_parameters = test_lame_parameters()
        .chain(test_invsicid_parameters())
        .cycle()
        .take(n)
        .map(Into::into)
        .collect::<Vec<_>>();
    let positions: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(n * 4)
        .collect();
    let positions: Vec<Vector4<f32>> = positions
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .map(|p| p.xzy().push(0.))
        .collect();
    let position_gradients = test_position_gradients_random(n)
        .into_iter()
        .map(|m| stack![m; Matrix1x3::zeros()])
        .collect::<Vec<_>>();
    let velocities: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(n * 4)
        .collect();
    let velocities: Vec<Vector4<f32>> = velocities
        .chunks_exact(4)
        .map(Vector4::from_column_slice)
        .map(|p| p.xzy().push(0.))
        .collect();
    let velocity_gradients: Vec<f32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<f32>()
        .take(n * 12)
        .collect();
    let velocity_gradients: Vec<Matrix4x3<f32>> = velocity_gradients
        .chunks_exact(12)
        .map(Matrix4x3::from_column_slice)
        .collect();

    check(InputData {
        permutation: &permutation,
        indices: &indices,
        masses: &masses,
        initial_volumes: &initial_volumes,
        parameters: &particle_parameters,
        positions: &positions,
        position_gradients: &position_gradients,
        velocities: &velocities,
        velocity_gradients: &velocity_gradients,
    });
}

fn run_permute_particles(
    workgroup_size: NonZeroU32,
    dispatch_limit: NonZeroU32,
    input_data: InputData,
) -> (
    Vec<u32>,
    Vec<f32>,
    Vec<f32>,
    Vec<particle_parameters::Device>,
    Vec<Vector4<f32>>,
    Vec<Matrix4x3<f32>>,
    Vec<Vector4<f32>>,
    Vec<Matrix4x3<f32>>,
) {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), workgroup_size, dispatch_limit, input_data);
    let permute_particles =
        PermuteParticles::new(&mut context, Settings { workgroup_size }).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output {
        indices_out,
        masses_out,
        initial_volumes_out,
        parameters_out,
        positions_out,
        position_gradients_out,
        velocities_out,
        velocity_gradients_out,
    } = permute_particles
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let downloads = DownloadsToHost::new(
        &context,
        [
            indices_out,
            masses_out,
            initial_volumes_out,
            parameters_out,
            positions_out,
            position_gradients_out,
            velocities_out,
            velocity_gradients_out,
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
        indices_out,
        masses_out,
        initial_volumes_out,
        parameters_out,
        positions_out,
        position_gradients_out,
        velocities_out,
        velocity_gradients_out,
    ] = downloads.try_into().unwrap();

    let mut garbage_w: Vec<Vector4<f32>> = positions_out.to_vec();
    garbage_w.iter_mut().for_each(|v| v.w = 0.);

    (
        indices_out.to_vec(),
        masses_out.to_vec(),
        initial_volumes_out.to_vec(),
        parameters_out.to_vec(),
        positions_out.to_vec(),
        position_gradients_out.to_vec(),
        velocities_out.to_vec(),
        velocity_gradients_out.to_vec(),
    )
}
