// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub(crate) mod cache;
mod collider;
mod compute_thread;
#[allow(unused)]
mod elastic;
mod error_messages;
mod fluid;
mod grids;
mod interpolate;
mod kinematic;
mod particles;
mod simulation_local;
mod solid;
mod state;

pub use interpolate::weights;
pub use simulation_local::SimulationLocal;
pub use state::{Phase, PhaseInput, State};
