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

use rand::prelude::*;
use rand::rngs::ChaCha8Rng;

use super::*;

fn expected(
    source: &[u32],
    mask: Option<&[u32]>,
    target: &[u32],
    Parameters {
        offset,
        stride,
        first,
        words,
    }: Parameters,
) -> Vec<u32> {
    let mut expected = target.to_vec();
    for (index, value) in source.iter().enumerate() {
        let (element, word) = (index / words as usize, index % words as usize);
        let bits = mask.map_or(u32::MAX, |mask| mask[index]);
        let destination =
            &mut expected[(offset as usize + element) * stride as usize + first as usize + word];
        *destination = (*destination & !bits) | (value & bits);
    }
    expected
}

fn random_words(rng: &mut impl Rng, len: usize) -> Vec<u32> {
    rng.random_iter().take(len).collect()
}

fn parameters(offset: u32, stride: u32, first: u32, words: u32) -> Parameters {
    Parameters {
        offset,
        stride,
        first,
        words,
    }
}

#[test]
fn writes_only_the_selected_bits_of_the_selected_words_and_elements() {
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    for (parameters, masked) in [
        (parameters(0, 1, 0, 1), false),
        (parameters(13, 4, 3, 1), false),
        (parameters(100, 4, 0, 3), false),
        (parameters(1, 9, 4, 1), false),
        (parameters(57, 12, 0, 12), false),
        (parameters(0, 1, 0, 1), true),
        (parameters(42, 1, 0, 1), true),
        (parameters(7, 4, 1, 2), true),
    ] {
        let target = random_words(&mut rng, 300 * parameters.stride as usize);
        let source = random_words(&mut rng, 200 * parameters.words as usize);
        let mask = masked.then(|| random_words(&mut rng, source.len()));

        let result = run(
            &source,
            mask.as_deref(),
            &target,
            parameters,
            GpuStatus::reached_frame_time(),
        );

        assert_eq!(
            result,
            expected(&source, mask.as_deref(), &target, parameters),
            "{parameters:?}, masked: {masked}"
        );
    }
}

#[test]
fn writes_nothing_before_the_frame_time_is_reached() {
    let mut rng = ChaCha8Rng::seed_from_u64(8);
    let target = random_words(&mut rng, 40);
    let source = random_words(&mut rng, 10);
    let mask = vec![u32::MAX; source.len()];

    for mask in [None, Some(mask.as_slice())] {
        let result = run(
            &source,
            mask,
            &target,
            parameters(0, 4, 0, 1),
            GpuStatus::default(),
        );

        assert_eq!(result, target, "masked: {}", mask.is_some());
    }
}

fn run(
    source: &[u32],
    mask: Option<&[u32]>,
    target: &[u32],
    parameters: Parameters,
    status: GpuStatus,
) -> Vec<u32> {
    let mut context = get_shared_context();

    let input = Input {
        source: Allocation::new(context.device(), "source", source).unwrap(),
        mask: mask.map(|mask| Allocation::new(context.device(), "mask", mask).unwrap()),
        target: Allocation::new(context.device(), "target", target).unwrap(),
    };
    let target = input.target.clone();

    let write_words = WriteWords::new(
        &mut context,
        Settings {
            workgroup_size: 64.try_into().unwrap(),
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
        },
    )
    .unwrap();
    let mut encoder = context.device().create_command_encoder(&Default::default());

    context.set_status(status).unwrap();

    let Output = write_words
        .record(&mut context, &mut (&mut encoder).into(), input, parameters)
        .unwrap();

    let download = DownloadToHost::new(&context, target);
    download.copy(&mut encoder);

    context.queue().submit([encoder.finish()]);
    let download = download.prep();
    context
        .device()
        .poll(wgpu::PollType::wait_indefinitely())
        .unwrap();

    download.to_vec().unwrap()
}
