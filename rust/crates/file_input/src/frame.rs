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

use squishy_volumes_api::InputBulk;
#[cfg(test)]
use squishy_volumes_util::AnimatedGlobals;

use crate::{BulkAttributeError, FrameVerificationError};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub struct FrameBulk<'a> {
    pub meta: FrameBulkMeta,
    pub data: InputBulk<'a>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub struct FrameBulkMeta {
    pub object_name: String,
    pub captured_attribute: BulkAttribute,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum BulkAttribute {
    Particles(FrameBulkParticles),
    Collider(FrameBulkCollider),
}

impl BulkAttribute {
    fn elem_count(&self) -> usize {
        match self {
            Self::Particles(inner) => inner.elem_count(),
            Self::Collider(inner) => inner.elem_count(),
        }
    }

    fn data_type(&self) -> &'static str {
        match self {
            Self::Particles(
                FrameBulkParticles::Flags
                | FrameBulkParticles::ColliderBits
                | FrameBulkParticles::Exponent,
            )
            | Self::Collider(FrameBulkCollider::Triangles) => INT,
            Self::Particles(_) | Self::Collider(_) => FLOAT,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    PartialOrd,
    strum_macros::EnumIter,
)]
pub enum FrameBulkParticles {
    Flags,
    ColliderBits,
    Transform,
    Size,
    Density,
    YoungsModulus,
    PoissonsRatio,
    InitialPosition,
    Velocity,
    ViscosityDynamic,
    ViscosityBulk,
    Exponent,
    BulkModulus,
    SandAlpha,
    GoalPosition,
}

impl FrameBulkParticles {
    fn elem_count(&self) -> usize {
        match self {
            Self::ColliderBits
            | Self::Size
            | Self::Density
            | Self::YoungsModulus
            | Self::PoissonsRatio
            | Self::ViscosityDynamic
            | Self::ViscosityBulk
            | Self::Exponent
            | Self::BulkModulus
            | Self::SandAlpha => 1,
            Self::Flags => 2,
            Self::InitialPosition | Self::Velocity | Self::GoalPosition => 3,
            Self::Transform => 16,
        }
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    PartialEq,
    PartialOrd,
    strum_macros::EnumIter,
)]
pub enum FrameBulkCollider {
    Position,
    Triangles,
    Friction,
}

impl FrameBulkCollider {
    fn elem_count(&self) -> usize {
        match self {
            FrameBulkCollider::Position | FrameBulkCollider::Triangles => 3,
            FrameBulkCollider::Friction => 1,
        }
    }
}

#[cfg(test)]
use std::borrow::Cow;

#[cfg(test)]
pub fn random_particle_bulk(
    object_name: String,
    num_particles: u32,
    rng: &mut impl rand::Rng,
) -> Vec<FrameBulk<'static>> {
    use rand::RngExt as _;
    vec![FrameBulk {
        meta: FrameBulkMeta {
            object_name,
            captured_attribute: BulkAttribute::Particles(FrameBulkParticles::Flags),
        },
        data: InputBulk::Ints(Cow::Owned(
            rng.random_iter::<i32>()
                .take(2 * num_particles as usize)
                .collect(),
        )),
    }]
}

#[cfg(test)]
pub fn random_collider_bulk(
    object_name: String,
    num_vertices: u32,
    num_triangles: u32,
    rng: &mut impl rand::Rng,
) -> Vec<FrameBulk<'static>> {
    use rand::RngExt as _;
    vec![
        FrameBulk {
            meta: FrameBulkMeta {
                object_name: object_name.clone(),
                captured_attribute: BulkAttribute::Collider(FrameBulkCollider::Position),
            },
            data: InputBulk::Floats(Cow::Owned(
                rng.random_iter::<f32>()
                    .take(3 * num_vertices as usize)
                    .collect(),
            )),
        },
        FrameBulk {
            meta: FrameBulkMeta {
                object_name: object_name.clone(),
                captured_attribute: BulkAttribute::Collider(FrameBulkCollider::Triangles),
            },
            data: InputBulk::Ints(Cow::Owned(
                rng.random_iter::<i32>()
                    .take(3 * num_triangles as usize)
                    .collect(),
            )),
        },
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct InputFrame {
    pub animated_globals: squishy_volumes_util::AnimatedGlobals,
    pub bulk: Vec<OwnedFrameBulk>,
}

impl FrameBulk<'_> {
    pub fn verify(&self, header: &crate::Header) -> Result<(), crate::FrameVerificationError> {
        let num = match &self.meta.captured_attribute {
            BulkAttribute::Particles(_) => header
                .get_particle_object(&self.meta.object_name)?
                .particle_range
                .len(),
            BulkAttribute::Collider(FrameBulkCollider::Position) => header
                .get_collider_object(&self.meta.object_name)?
                .vertex_range
                .len(),
            BulkAttribute::Collider(FrameBulkCollider::Triangles)
            | BulkAttribute::Collider(FrameBulkCollider::Friction) => header
                .get_collider_object(&self.meta.object_name)?
                .triangle_range
                .len(),
        };

        self.verify_data(num).map_err(|error| {
            error
                .attach_attr(self.meta.captured_attribute)
                .attach_name(self.meta.object_name.clone())
        })
    }

    fn verify_data(&self, num: usize) -> Result<(), BulkAttributeError> {
        let attribute = self.meta.captured_attribute;

        let expected = attribute.data_type();
        let found = match self.data {
            InputBulk::Floats(_) => FLOAT,
            InputBulk::Ints(_) => INT,
        };
        if expected != found {
            return Err(BulkAttributeError::TypeMismatch { expected, found });
        }

        let found = self.data.len();
        let expected = num * attribute.elem_count();
        if expected != found {
            return Err(BulkAttributeError::LengthMismatch { found, expected });
        }

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum OwnedInputBulk {
    Floats(Vec<f32>),
    Ints(Vec<i32>),
}

impl From<InputBulk<'_>> for OwnedInputBulk {
    fn from(value: InputBulk) -> Self {
        match value {
            InputBulk::Floats(cow) => Self::Floats(cow.into_owned()),
            InputBulk::Ints(cow) => Self::Ints(cow.into_owned()),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub struct OwnedFrameBulk {
    pub meta: FrameBulkMeta,
    pub data: OwnedInputBulk,
}

impl From<FrameBulk<'_>> for OwnedFrameBulk {
    fn from(FrameBulk { meta, data }: FrameBulk<'_>) -> Self {
        Self {
            meta,
            data: data.into(),
        }
    }
}

const FLOAT: &str = "float";
const INT: &str = "int";

impl OwnedFrameBulk {
    fn verification_error(&self, error: BulkAttributeError) -> FrameVerificationError {
        error
            .attach_attr(self.meta.captured_attribute)
            .attach_name(self.meta.object_name.clone())
    }

    #[inline]
    pub fn assume_floats<T: bytemuck::Pod>(&self) -> Result<&[T], FrameVerificationError> {
        let expected = FLOAT;
        (|| {
            let found = match &self.data {
                OwnedInputBulk::Floats(vec) => {
                    return Ok(bytemuck::try_cast_slice(vec.as_slice())?);
                }
                OwnedInputBulk::Ints(_) => INT,
            };
            Err(BulkAttributeError::TypeMismatch { expected, found })
        })()
        .map_err(|error| self.verification_error(error))
    }

    #[inline]
    pub fn assume_ints<T: bytemuck::Pod>(&self) -> Result<&[T], FrameVerificationError> {
        let expected = INT;
        (|| {
            let found = match &self.data {
                OwnedInputBulk::Floats(_) => FLOAT,
                OwnedInputBulk::Ints(vec) => return Ok(bytemuck::try_cast_slice(vec.as_slice())?),
            };
            Err(BulkAttributeError::TypeMismatch { expected, found })
        })()
        .map_err(|error| self.verification_error(error))
    }
}

#[cfg(test)]
impl InputFrame {
    pub fn test_input_0(
        num_particles: u32,
        num_vertices: u32,
        num_triangles: u32,
    ) -> (AnimatedGlobals, Vec<FrameBulk<'static>>) {
        use rand::{SeedableRng, rngs::ChaCha8Rng};
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut bulk = Vec::new();
        bulk.append(&mut random_particle_bulk(
            "foo".to_string(),
            num_particles,
            &mut rng,
        ));
        bulk.append(&mut random_particle_bulk(
            "bar".to_string(),
            num_particles,
            &mut rng,
        ));
        bulk.append(&mut random_collider_bulk(
            "car".to_string(),
            num_vertices,
            num_triangles,
            &mut rng,
        ));
        (
            AnimatedGlobals {
                gravity_x: 1.,
                gravity_y: 2.,
                gravity_z: 3.,
                goal_stiffness: 42.,
                goal_damping: 0.5,
                damping: 1.23,
            },
            bulk,
        )
    }

    pub fn test_input_1(num_particles: u32) -> (AnimatedGlobals, Vec<FrameBulk<'static>>) {
        use rand::{SeedableRng, rngs::ChaCha8Rng};
        let mut rng = ChaCha8Rng::seed_from_u64(69);
        let mut bulk = Vec::new();
        bulk.append(&mut random_particle_bulk(
            "foo".to_string(),
            num_particles,
            &mut rng,
        ));
        bulk.append(&mut random_particle_bulk(
            "bar".to_string(),
            num_particles,
            &mut rng,
        ));
        bulk.append(&mut random_particle_bulk(
            "car".to_string(),
            num_particles,
            &mut rng,
        ));

        (
            AnimatedGlobals {
                gravity_x: 2.,
                gravity_y: 3.,
                gravity_z: 4.,
                goal_stiffness: 43.,
                goal_damping: 0.8,
                damping: 1.2,
            },
            bulk,
        )
    }

    pub fn test_input_2() -> (AnimatedGlobals, Vec<FrameBulk<'static>>) {
        (
            AnimatedGlobals {
                gravity_x: 3.,
                gravity_y: 4.,
                gravity_z: 5.,
                goal_stiffness: 44.,
                goal_damping: 0.1,
                damping: 0.,
            },
            Default::default(),
        )
    }
}
