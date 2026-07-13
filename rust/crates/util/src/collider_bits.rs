// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
