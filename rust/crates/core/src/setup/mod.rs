// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

// The setup is processed input ready to be used in the simulation.
//
// Because there can be animated inputs,
// the setup needs to store two frames worth of input.

pub mod animatable;
pub mod constant;
pub mod mesh;
pub mod serialization;
pub mod serialized_vector;
pub mod setup;

pub use animatable::*;
pub use constant::*;
pub use mesh::*;
pub use serialization::*;
pub use serialized_vector::*;
pub use setup::*;
