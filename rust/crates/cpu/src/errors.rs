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

    #[error("Something went wrong accessing frame input")]
    FrameInput(#[from] squishy_volumes_xpu::FrameInputError),

    #[error("The time step ended up being 0")]
    ZeroTimeStep,

    #[error(
        "Triangle {triangle} of collider '{object}' is degenerate (zero area) \
         between input frames {frame} and {}",
        frame + 1
    )]
    DegenerateTriangle {
        object: String,
        triangle: usize,
        frame: usize,
    },

    #[error("Something went wrong with the harness")]
    HarnessError(#[from] squishy_volumes_xpu::HarnessError),

    #[error("Failed to compute the elastic energy of a particle")]
    EnergyError(#[from] squishy_volumes_util::EnergyError),
}
