// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

fn check(new_flags: &[ParticleFlags], flags: &[ParticleFlags]) {
    let gpu_flags = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        new_flags,
        flags,
    );

    let cpu_flags: Vec<ParticleFlags> = new_flags
        .iter()
        .zip(flags)
        .map(|(new_flags, flags)| {
            let mut f = *flags;
            f.set(
                ParticleFlags::HAS_GOAL,
                new_flags.contains(ParticleFlags::HAS_GOAL),
            );
            f
        })
        .collect();

    for (cpu, gpu) in cpu_flags.into_iter().zip(gpu_flags) {
        assert_eq!(cpu, gpu);
    }
}

#[test]
fn simple() {
    check(
        &[ParticleFlags::IS_SOLID | ParticleFlags::USE_VISCOSITY | ParticleFlags::TOMBSTONED],
        &[ParticleFlags::HAS_GOAL],
    );
}

fn run(
    settings: Settings,
    new_falgs: &[ParticleFlags],
    flags: &[ParticleFlags],
) -> Vec<ParticleFlags> {
    let mut context = SHARED_CONTEXT.lock().unwrap();

    let input = Input::new(context.device(), new_falgs, flags).unwrap();
    let flags = input.flags.clone();

    let update_flags = UpdateFlags::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output = update_flags
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, flags);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
