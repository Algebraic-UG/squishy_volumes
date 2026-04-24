// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod cache;

mod api_impl;
pub use api_impl::*;

mod compute_thread;
mod directory_lock;
mod input_file;
mod input_interpolation;
pub mod kernels;
mod phase;
mod rasterization;
mod report;
mod state;
mod stats;

pub use report::{Report, ReportInfo};

#[cfg(feature = "profile")]
use coarse_prof::profile;
#[cfg(not(feature = "profile"))]
macro_rules! profile {
    ($name:expr) => {};
}
#[cfg(not(feature = "profile"))]
use profile;
