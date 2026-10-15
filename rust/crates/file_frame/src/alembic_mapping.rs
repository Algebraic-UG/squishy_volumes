// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub const APPLICATION_KEY: &str = "_ai_Application";
pub const SQUISHY_VOLUMES: &str = "Squishy Volumes";
pub const SQUISHY_VOLUMES_BLOB: &str = "Squishy Volumes Blob";

pub use alembic_rust::POSITIONS;
pub use alembic_rust::VELOCITIES;

pub const FLAGS: &str = "flags";
pub const DENSITY: &str = "density";
pub const INITIAL_VOLUME: &str = "initial_volume";
pub const VISCOSITY_DYNAMIC: &str = "viscosity_dynamic";
pub const VISCOSITY_BULK: &str = "viscosity_bulk";
pub const YOUNGS_MODULUS: &str = "youngs_modulus";
pub const POISSONS_RATIO: &str = "poissons_ratio";
pub const SAND_ALPHA: &str = "sand_alpha";
pub const BULK_MODULUS: &str = "bulk_modulus";
pub const EXPONENT: &str = "exponent";
pub const ELASTIC_ENERGIES: &str = "elastic_energies";
pub const COLLIDER_BITS: &str = "collider_bits";
pub const POSITION_GRADIENTS_X: &str = "position_gradients_x";
pub const POSITION_GRADIENTS_Y: &str = "position_gradients_y";
pub const POSITION_GRADIENTS_Z: &str = "position_gradients_z";
pub const VELOCITY_GRADIENTS_X: &str = "velocity_gradients_x";
pub const VELOCITY_GRADIENTS_Y: &str = "velocity_gradients_y";
pub const VELOCITY_GRADIENTS_Z: &str = "velocity_gradients_z";
pub const INITIAL_POSITIONS: &str = "initial_positions";

pub struct AlembicData<'a> {
    pub elastic_energies: &'a mut [f32],
    pub positions: &'a mut [[f32; 3]],
    pub velocities: &'a mut [[f32; 3]],
    pub initial_positions: &'a mut [[f32; 3]],
    pub flags: &'a mut [[f32; 2]],
    pub split_parameters: &'a mut squishy_volumes_util::SplitParameters,
    pub collider_bits: &'a mut [[f32; 2]],
    pub position_gradients_x: &'a mut [[f32; 3]],
    pub position_gradients_y: &'a mut [[f32; 3]],
    pub position_gradients_z: &'a mut [[f32; 3]],
    pub velocity_gradients_x: &'a mut [[f32; 3]],
    pub velocity_gradients_y: &'a mut [[f32; 3]],
    pub velocity_gradients_z: &'a mut [[f32; 3]],
}
