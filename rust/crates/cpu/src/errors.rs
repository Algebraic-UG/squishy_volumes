// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Something went wron accessing frame input: {0}")]
    FrameInput(#[from] squishy_volumes_xpu::FrameInputError),

    #[error("Failed to cast a vector")]
    CastFailed,

    #[error("At this point, interpolated input should be ready")]
    InterpolatedInputMissing,

    #[error("The time step ended up being 0")]
    ZeroTimeStep,

    #[error("Something went wrong with the harness: {0}")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("The grid is missing is the serialization")]
    GridMissing,
    #[error("The grid node is missing is the serialization")]
    GridNodeMissing,
}
