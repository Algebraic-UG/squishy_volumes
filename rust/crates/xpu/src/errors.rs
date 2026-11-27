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

use crate::FrameInputError;

#[derive(thiserror::Error, Debug)]
pub enum SolverError {
    #[error("failed to read the particles from the state")]
    ReadState(#[source] IoStateError),
    #[error("failed to store the particles in the state")]
    StoreState(#[source] IoStateError),

    #[error("failed to prepare the input frames")]
    PrepareInput(#[source] FrameInputError),
    #[error("failed to load the next input frame")]
    LoadNextInput(#[source] FrameInputError),
    #[error("failed to apply the next input frame")]
    ApplyNextInput(#[source] squishy_volumes_file_input::Error),
}
