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

use super::*;

fn check(
    flags: &[ParticleFlags],
    new_flags: &[ParticleFlags],
    parameters @ Parameters { offset }: Parameters,
) {
    let gpu_flags = run(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
        new_flags,
        flags,
        parameters,
    );

    let cpu_flags: Vec<ParticleFlags> = new_flags
        .iter()
        .zip(flags.iter().skip(offset as usize))
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
        println!("{:032b}", cpu);
        println!("{:032b}", gpu);
        println!("--------------");
        assert_eq!(cpu, gpu);
    }
}

#[test]
fn simple() {
    check(
        &[ParticleFlags::IS_SOLID | ParticleFlags::USE_VISCOSITY | ParticleFlags::TOMBSTONED],
        &[ParticleFlags::HAS_GOAL],
        Parameters { offset: 0 },
    );
}

fn run(
    settings: Settings,
    new_falgs: &[ParticleFlags],
    flags: &[ParticleFlags],
    parameters: Parameters,
) -> Vec<ParticleFlags> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), new_falgs, flags).unwrap();
    let flags = input.flags.clone();

    let update_flags = UpdateFlags::new(&mut context, settings).unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    context.set_status(GpuStatus::reached_frame_time()).unwrap();

    let Output = update_flags
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
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
