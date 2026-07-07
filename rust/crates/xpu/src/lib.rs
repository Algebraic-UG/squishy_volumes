// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod errors;
mod frame_input;
mod harness;

pub use errors::*;
pub use frame_input::*;
pub use harness::*;

pub trait XpuState: std::marker::Sized {
    type Error;

    fn produce_next_state(
        &mut self,
        harness: &mut Harness,
        frame_input: &mut FrameInput,
    ) -> Result<squishy_volumes_file_frame::IoState, Self::Error>;
}
