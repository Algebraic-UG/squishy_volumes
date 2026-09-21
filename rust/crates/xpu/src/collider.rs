// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Default, Debug, Clone)]
pub struct Collider {
    pub vertex_positions: Vec<nalgebra::Vector3<f32>>,
    pub triangle_frictions: Vec<f32>,
    pub triangle_dampings: Vec<f32>,
}

impl TryFrom<squishy_volumes_file_frame::Collider> for Collider {
    type Error = crate::FrameInputError;

    fn try_from(
        squishy_volumes_file_frame::Collider {
            vertex_positions,
            triangle_frictions,
            triangle_dampings,
        }: squishy_volumes_file_frame::Collider,
    ) -> Result<Self, Self::Error> {
        Ok(Collider {
            vertex_positions: bytemuck::try_cast_vec(vertex_positions).map_err(|(e, _)| e)?,
            triangle_frictions,
            triangle_dampings,
        })
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
