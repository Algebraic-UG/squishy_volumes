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
    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 0;
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys = [0, 3, 2, 2, 3, 2, 0, 3, 2, 1];
    let mut indices: Vec<_> = (0..keys.len() as u32).collect();

    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        bit_count.get(),
        bit_offset,
        workgroup_size.get(),
        subgroup_size.get(),
        &indices,
        &keys,
    );
    let prefix_sums = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count.get(), bit_offset, &indices, &keys),
        run_reorder_indices(settings, parameters, None, &keys, &prefix_sums),
    );

    shuffle(&mut indices, 5);

    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        bit_count.get(),
        bit_offset,
        workgroup_size.get(),
        subgroup_size.get(),
        &indices,
        &keys,
    );
    let prefix_sums = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count.get(), bit_offset, &indices, &keys),
        run_reorder_indices(settings, parameters, Some(&indices), &keys, &prefix_sums),
    );
}

#[test]
fn test_random() {
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    let workgroup_size = 64.try_into().unwrap();
    let dispatch_limit = (u16::MAX as u32).try_into().unwrap();
    let bit_count = 2.try_into().unwrap();
    let settings = Settings {
        workgroup_size,
        dispatch_limit,
        bit_count,
    };
    let bit_offset = 0;
    let parameters = Parameters { bit_offset };
    let subgroup_size = get_subgroup_size();

    let keys: Vec<u32> = ChaCha8Rng::seed_from_u64(42)
        .random_iter()
        .take(1000)
        .collect();
    let mut indices: Vec<_> = (0..keys.len() as u32).collect();

    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        bit_count.get(),
        bit_offset,
        workgroup_size.get(),
        subgroup_size.get(),
        &indices,
        &keys,
    );
    let prefix_sums = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count.get(), bit_offset, &indices, &keys),
        run_reorder_indices(settings, parameters, None, &keys, &prefix_sums),
    );

    shuffle(&mut indices, 6);

    let counts = count_subkeys_on_cpu(
        dispatch_limit.get(),
        bit_count.get(),
        bit_offset,
        workgroup_size.get(),
        subgroup_size.get(),
        &indices,
        &keys,
    );
    let prefix_sums = prefix_sum_on_cpu(&counts);

    assert_eq!(
        sort_on_cpu_by_bits(bit_count.get(), bit_offset, &indices, &keys),
        run_reorder_indices(settings, parameters, Some(&indices), &keys, &prefix_sums),
    );
}

fn run_reorder_indices(
    settings: Settings,
    parameters: Parameters,
    indices: Option<&[u32]>,
    keys: &[u32],
    prefix_sums: &[u32],
) -> Vec<u32> {
    let mut context = get_shared_context();

    let input = Input::new(context.device(), settings, indices, keys, prefix_sums).unwrap();

    let reorder_indices = ReorderIndices::new(&mut context, settings).unwrap();

    let mut encoder = context.device().create_command_encoder(&Default::default());

    let Output { indices_out } = reorder_indices
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, indices_out);

    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
