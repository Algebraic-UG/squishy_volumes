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
#[allow(unused)]
mod elastic;
mod error_messages;
mod input_file;
pub mod kernels;
mod math;
mod mesh;
mod report;
//mod setup;
mod phase;
mod state;
mod stats;

pub use report::{Report, ReportInfo};

// TODO: this might be better somewhere else.
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
#[cfg(not(feature = "profile"))]
macro_rules! profile {
    ($name:expr) => {};
}
#[cfg(not(feature = "profile"))]
use profile;
