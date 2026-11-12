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

mod aabb;
mod animated_globals;
mod attributes;
pub mod collider_bits;
mod collision;
mod consts;
mod elastic;
mod external_force;
mod flat;
mod fluid;
mod limit_time_step;
mod panic_to_string;
mod particle_flags;
mod particle_parameters;
mod safe_inverse;
mod sand;
pub mod test_data;
mod typedefs;

pub use aabb::*;

pub use animated_globals::*;
pub use attributes::*;
pub use collision::*;
pub use consts::*;
pub use elastic::*;
pub use external_force::*;
pub use flat::*;
pub use fluid::*;
pub use limit_time_step::*;
pub use panic_to_string::*;
pub use particle_flags::*;
pub use particle_parameters::*;
pub use safe_inverse::*;
pub use sand::*;
pub use typedefs::*;

#[cfg(use_f64)]
type T = f64;
#[cfg(all(use_f64, test))]
mod physics_tests;
#[cfg(all(use_f64, test))]
mod tests;

#[cfg(not(use_f64))]
type T = f32;

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
pub use coarse_prof;
#[cfg(feature = "profile")]
pub use coarse_prof::profile;

#[macro_export]
macro_rules! fake_profile {
    ($name:expr) => {};
}
#[cfg(not(feature = "profile"))]
pub use fake_profile as profile;
