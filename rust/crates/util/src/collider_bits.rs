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

fn near(bits: u32, collider: usize) -> bool {
    bits & (0x0001_0000 << collider) != 0
}

fn side(bits: u32, collider: usize) -> bool {
    bits & (0x0000_0001 << collider) != 0
}

pub fn get(bits: u32, collider: usize) -> Option<bool> {
    near(bits, collider).then_some(side(bits, collider))
}

pub fn set(bits: &mut u32, collider: usize, side: Option<bool>) {
    *bits &= !(0x0001_0001 << collider);
    match side {
        Some(true) => {
            *bits |= 0x0001_0001 << collider;
        }
        Some(false) => {
            *bits |= 0x0001_0000 << collider;
        }
        None => {}
    }
}

pub fn compatible(a: &u32, b: &u32) -> bool {
    let mask = (a & b) >> 16;
    let diff = a ^ b;
    (mask & diff) == 0
}
