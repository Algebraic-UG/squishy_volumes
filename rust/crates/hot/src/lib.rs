// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub use build_info::BuildInfo;

build_info::build_info!(fn _build_info);

#[unsafe(no_mangle)]
pub fn build_info() -> BuildInfo {
    _build_info().clone()
}

pub use squishy_volumes_api::{Context, Simulation, Task};
use squishy_volumes_core::ContextImpl;

#[unsafe(no_mangle)]
pub fn create_context() -> Box<dyn Context> {
    Box::new(ContextImpl::default())
}
