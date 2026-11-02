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

use std::sync::Mutex;

use nalgebra::Vector3;
use rustc_hash::FxHashMap;
use smallvec::SmallVec;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct GridKey {
    pub node_id: Vector3<i32>,
    pub collider_bits: u32,
}

#[derive(Default)]
pub struct GridNodes {
    pub map: FxHashMap<GridKey, u32>,

    pub multi_map: FxHashMap<Vector3<i32>, SmallVec<[u32; 3]>>,

    pub keys: Vec<GridKey>,

    pub contributors: Vec<Mutex<SmallVec<[u32; 16]>>>,

    pub masses: Vec<f32>,
    pub velocities: Vec<Vector3<f32>>,
}
