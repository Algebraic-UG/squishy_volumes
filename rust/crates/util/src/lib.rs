// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod aabb;
pub mod bounding_volume_hierarchy;
mod consts;
mod elastic;
mod flat;
pub mod rasterization;
mod safe_inverse;
pub mod triangle;
mod typedefs;

pub use aabb::Aabb;
pub use bounding_volume_hierarchy::BoundingVolumeHierarchy;

pub use consts::*;
pub use elastic::*;
pub use flat::*;
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
