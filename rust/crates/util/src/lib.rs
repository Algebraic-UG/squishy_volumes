// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod aabb;
pub mod collider_bits;
mod consts;
mod elastic;
mod flat;
mod panic_to_string;
mod safe_inverse;
mod typedefs;

pub use aabb::*;

pub use consts::*;
pub use elastic::*;
pub use flat::*;
pub use panic_to_string::*;
pub use safe_inverse::*;
pub use typedefs::*;

#[cfg(test)]
type T = f64;
#[cfg(not(test))]
type T = f32;

#[cfg(test)]
mod tests;

#[macro_export]
macro_rules! ensure_err {
    ($cond:expr, $err:expr $(,)?) => {
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !$cond {
            return Err($err);
        }
    };
}

#[cfg(feature = "profile")]
use coarse_prof::profile;
#[macro_export]
macro_rules! fake_profile {
    ($name:expr) => {};
}
#[cfg(not(feature = "profile"))]
pub use fake_profile as profile;
