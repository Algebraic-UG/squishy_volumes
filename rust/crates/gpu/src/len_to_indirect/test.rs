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

use rand::{RngExt as _, SeedableRng as _, rngs::ChaCha8Rng};

use super::*;

fn check(
    settings @ Settings {
        workgroup_size,
        dispatch_limit,
    }: Settings,
    numbers: &[u32],
) {
    let len = numbers.iter().sum();

    let indirect = Indirect::new(DispatchSettings {
        workgroup_size,
        dispatch_limit,
        len,
    });

    assert_eq!(vec![indirect], run(settings, len),);
}

#[test]
fn test_simple() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = 4.try_into().unwrap();

    let numbers = [0, 1, 1, 1, 1, 1, 0, 1, 1, 1];
    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &numbers,
    );
}

#[test]
fn test_random() {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = 4.try_into().unwrap();

    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<bool>()
        .map(|bit| if bit { 1 } else { 0 })
        .take(10000)
        .collect();
    check(
        Settings {
            workgroup_size,
            dispatch_limit,
        },
        &numbers,
    );
}

fn run(settings: Settings, len: u32) -> Vec<Indirect> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), len).unwrap();
    let indirect = input.indirect.clone();
    let len_to_indirect = LenToIndirect::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output = len_to_indirect
        .record(
            &mut context,
            &mut (&mut encoder).into(),
            input,
            Parameters { limit: len },
        )
        .unwrap();

    let download = DownloadToHost::new(&context, indirect);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
