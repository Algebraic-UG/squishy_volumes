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

#[test]
fn single() {
    let bits = vec![0b0001001010];
    let pops = vec![3];

    assert_eq!(
        pops,
        run(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
            },
            &bits,
        )
    );
}

#[test]
fn random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let bits: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter::<u32>()
        .take(1000)
        .collect();
    let pops: Vec<u32> = bits.iter().map(|b| b.count_ones()).collect();

    assert_eq!(
        pops,
        run(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
            },
            &bits,
        )
    );
}

fn run(settings: Settings, bits: &[u32]) -> Vec<u32> {
    let mut context = get_shared_context();

    let input = Input::new(
        context.device(),
        settings,
        (u16::MAX as u32).try_into().unwrap(),
        bits,
    )
    .unwrap();
    let bits_to_pops = BitsToPops::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { pops } = bits_to_pops
        .record(&mut context, &mut (&mut encoder).into(), input, Parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, pops);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);

    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
