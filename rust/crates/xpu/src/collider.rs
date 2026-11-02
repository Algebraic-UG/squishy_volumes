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

#[derive(Default, Debug, Clone)]
pub struct Collider {
    pub vertex_positions: Vec<nalgebra::Vector3<f32>>,
    pub triangle_frictions: Vec<f32>,
    pub triangle_dampings: Vec<f32>,
}

impl From<squishy_volumes_file_frame::Collider> for Collider {
    fn from(
        squishy_volumes_file_frame::Collider {
            vertex_positions,
            triangle_frictions,
            triangle_dampings,
        }: squishy_volumes_file_frame::Collider,
    ) -> Self {
        Collider {
            vertex_positions: bytemuck::cast_vec(vertex_positions),
            triangle_frictions,
            triangle_dampings,
        }
    }
}

impl Collider {
    pub fn to_io_collider(&self) -> squishy_volumes_file_frame::Collider {
        squishy_volumes_file_frame::Collider {
            vertex_positions: bytemuck::cast_slice(&self.vertex_positions).to_vec(),
            triangle_frictions: self.triangle_frictions.clone(),
            triangle_dampings: self.triangle_dampings.clone(),
        }
    }
}
