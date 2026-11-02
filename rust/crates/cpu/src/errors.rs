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

use squishy_volumes_file_frame::IoStateError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Io state error")]
    IoState(#[from] IoStateError),

    #[error("The computation was canceled")]
    Canceled,

    #[error("Something went wrong accessing frame input")]
    FrameInput(#[from] squishy_volumes_xpu::FrameInputError),

    // TODO: this needs more debug info
    #[error("Failed to cast a vector")]
    CastFailed,

    #[error("At this point, interpolated input should be ready")]
    InterpolatedInputMissing,

    #[error("The time step ended up being 0")]
    ZeroTimeStep,

    #[error("Something went wrong with the harness")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("The grid is missing is the serialization")]
    GridMissing,
    #[error("The grid node is missing is the serialization")]
    GridNodeMissing,

    #[error("Failed to compute the elastic energy of a particle")]
    EnergyError(#[from] squishy_volumes_util::EnergyError),
}
