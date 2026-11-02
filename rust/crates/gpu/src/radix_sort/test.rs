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
fn test_simple() {
    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_radix_sort(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
                bit_count: 2.try_into().unwrap(),
            },
            None,
            &keys,
        )
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let indices: Vec<_> = (0..keys.len() as u32).collect();

    assert_eq!(
        sort_on_cpu(&indices, &keys),
        run_radix_sort(
            Settings {
                workgroup_size: 64.try_into().unwrap(),
                dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
                bit_count: 2.try_into().unwrap(),
            },
            None,
            &keys,
        )
    );
}

#[test]
fn specific() {
    let mut context = get_shared_context();

    let indirect = Indirect {
        x: 1,
        y: 1,
        z: 1,
        len: 3,
    };
    let keys = [2, 0, 1, 100, 200, 42];

    let indirect = Allocation::new(context.device(), "indirect", &[indirect]).unwrap();
    let keys = Allocation::new(context.device(), "keys", &keys).unwrap();

    let indices = run_with_input(
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            bit_count: 3.try_into().unwrap(),
        },
        &mut context,
        Input {
            indirect,
            indices_in: None,
            keys,
        },
        true,
    );
    println!("{indices:?}");
    assert!(
        (0..3)
            .into_iter()
            .all(|i| indices.as_slice()[0..3].contains(&i))
    );
}

fn run_radix_sort(settings: Settings, indices: Option<&[u32]>, keys: &[u32]) -> Vec<u32> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), settings.clone(), indices, keys).unwrap();
    run_with_input(settings, &mut context, input, false)
}

fn run_with_input(
    settings: Settings,
    context: &mut GpuContext,
    input: Input,
    one_round: bool,
) -> Vec<u32> {
    let radix_sort = RadixSort::new(context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let indices_out = if one_round {
        radix_sort
            .record(
                context,
                &mut (&mut encoder).into(),
                input,
                Parameters { bit_offset: 0 },
            )
            .unwrap()
            .indices_out
    } else {
        radix_sort
            .record_all_rounds(context, &mut (&mut encoder).into(), input)
            .unwrap()
    };

    let download = DownloadToHost::new(context, indices_out);
    download.copy(&mut encoder);
    context.queue().submit([encoder.finish()]);
    let download = download.prep();

    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
