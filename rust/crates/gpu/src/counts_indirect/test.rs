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

fn check(len: u32) {
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 3.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };

    let count_len = counts_count(CountsCountArgs {
        workgroup_size: workgroup_size.get(),
        subgroup_size: get_subgroup_size().get(),
        dispatch_limit: dispatch_limit.get(),
        counter: 2u32.pow(bit_count.get()),
        len,
    });

    let indirect = Indirect::new(DispatchSettings {
        workgroup_size,
        dispatch_limit,
        len: count_len,
    });
    assert_eq!(vec![indirect], run_counts_indirect(settings, len),);
}

#[test]
fn test_simple() {
    check(10)
}

#[test]
fn test_random() {
    let numbers: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .take(100)
        .collect();
    for number in numbers {
        check(number);
    }
}

fn run_counts_indirect(settings: Settings, len: u32) -> Vec<Indirect> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), settings, len).unwrap();
    let counts_indirect = CountsIndirect::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());
    let Output { indirect_counts } = counts_indirect
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, indirect_counts);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
