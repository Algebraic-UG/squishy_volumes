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

use std::collections::BTreeMap;

use squishy_volumes_util::ParticleFlags;

const SHADER_SOURCE: &str = include_str!("shaders/util/particle_flags.wesl");

fn parse_u32_expression(expression: &str) -> u32 {
    let parse = |literal: &str| {
        literal
            .trim()
            .trim_end_matches('u')
            .parse::<u32>()
            .unwrap_or_else(|_| panic!("unsupported flag expression `{expression}`"))
    };
    match expression.split_once("<<") {
        Some((value, shift)) => parse(value) << parse(shift),
        None => parse(expression),
    }
}

fn shader_flags() -> BTreeMap<String, u32> {
    SHADER_SOURCE
        .lines()
        .filter_map(|line| line.trim().strip_prefix("const "))
        .map(|declaration| {
            let (name, expression) = declaration
                .strip_suffix(';')
                .and_then(|declaration| declaration.split_once(": u32 ="))
                .unwrap_or_else(|| panic!("unsupported flag declaration `{declaration}`"));
            (name.to_string(), parse_u32_expression(expression))
        })
        .collect()
}

fn rust_flags() -> BTreeMap<String, u32> {
    ParticleFlags::all()
        .iter_names()
        .map(|(name, flag)| (name.to_string(), flag.bits()))
        .collect()
}

#[test]
fn shader_particle_flags_match_rust() {
    assert_eq!(shader_flags(), rust_flags());
}
