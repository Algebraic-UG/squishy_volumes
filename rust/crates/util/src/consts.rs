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

use super::T;

pub const NORMALIZATION_EPS: T = 1e-5;
pub const INVERSE_EPS: T = 1e-7;

pub const SINGULAR_VALUE_SEPARATION: T = 1e-5;

/*
pub const EIGEN_EPS: T = 1e-5;
pub const EIGEN_ITER: usize = 20;
pub const NEWTON_EPS: T = 0.1;
pub const NEWTON_ITER: usize = 10;
pub const CONJUGATE_GRADIENTS_EPS: T = 0.1;
pub const CONJUGATE_GRADIENTS_ITER: usize = 10;
pub const SAFE_CONDITION_VALUE: T = 1e-5;
*/
