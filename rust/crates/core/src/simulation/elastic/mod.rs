// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod energies;
#[cfg(all(test, feature = "f64"))]
mod tests;
#[cfg(all(test, not(feature = "f64")))]
#[test]
fn automatic_fail() {
    panic!("!!! ---> Run tests with 'f64' feature enabled <--- !!!")
}

pub use energies::*;
