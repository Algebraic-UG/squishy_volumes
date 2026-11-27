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

const IS_SOLID: u32 = 1;
const IS_FLUID: u32 = 1 << 1;
const USE_VISCOSITY: u32 = 1 << 2;
const USE_SAND_ALPHA: u32 = 1 << 3;
const HAS_GOAL: u32 = 1 << 4;
const IS_ACTIVE: u32 = 1 << 5;
const FAILED: u32 = 1 << 6;
