// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_api::InputBulk;
#[cfg(test)]
use squishy_volumes_util::AnimatedGlobals;

use crate::{AttributeError, InputObjectCollider, InputObjectParticles};

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
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkParticles {
    IsSolid,
    IsFluid,
    UseViscosity,
    UseSandAlpha,
    HasGoal,
    IsActive,
    ColliderBits,
    Transforms,
    Sizes,
    Densities,
    YoungsModuluses,
    PoissonsRatios,
    InitialPositions,
    InitialVelocity,
    ViscosityDynamic,
    ViscosityBulk,
    Exponent,
    BulkModulus,
    SandAlpha,
    GoalPositions,
}

impl FrameBulkParticles {
    fn elem_count(&self) -> usize {
        match self {
            Self::IsSolid
            | Self::IsFluid
            | Self::UseViscosity
            | Self::UseSandAlpha
            | Self::HasGoal
            | Self::IsActive
            | Self::ColliderBits
            | Self::Sizes
            | Self::Densities
            | Self::YoungsModuluses
            | Self::PoissonsRatios
            | Self::ViscosityDynamic
            | Self::ViscosityBulk
            | Self::Exponent
            | Self::BulkModulus
            | Self::SandAlpha => 1,
            Self::InitialPositions | Self::InitialVelocity | Self::GoalPositions => 3,
            Self::Transforms => 16,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkCollider {
    VertexPositions,
    Triangles,
    TriangleFrictions,
    TriangleDampings,
}

impl FrameBulkCollider {
    fn elem_count(&self) -> usize {
        match self {
            FrameBulkCollider::VertexPositions | FrameBulkCollider::Triangles => 3,
            FrameBulkCollider::TriangleFrictions | FrameBulkCollider::TriangleDampings => 1,
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
            captured_attribute: BulkAttribute::Particles(FrameBulkParticles::IsSolid),
        },
        data: InputBulk::Bool(Cow::Owned(
            rng.random_iter::<bool>()
                .take(num_particles as usize)
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
                captured_attribute: BulkAttribute::Collider(FrameBulkCollider::VertexPositions),
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
    // TODO: also verify the type
    pub fn verify(&self, header: &crate::InputHeader) -> Result<(), crate::FrameVerifcationError> {
        let header_obj = header.objects.get(&self.meta.object_name).ok_or(
            crate::ObjectError::ObjectNotInHeader.attach_name(self.meta.object_name.clone()),
        )?;

        let num =
            match (header_obj, &self.meta.captured_attribute) {
                (
                    crate::InputObject::Particles(InputObjectParticles { num_particles }),
                    BulkAttribute::Particles(_),
                ) => num_particles,
                (
                    crate::InputObject::Collider(InputObjectCollider {
                        num_vertices,
                        num_triangles,
                        ..
                    }),
                    BulkAttribute::Collider(frame_bulk_collider),
                ) => match frame_bulk_collider {
                    FrameBulkCollider::VertexPositions => num_vertices,
                    FrameBulkCollider::Triangles
                    | FrameBulkCollider::TriangleFrictions
                    | FrameBulkCollider::TriangleDampings => num_triangles,
                },
                _ => Err(crate::ObjectError::ObjectChangedType
                    .attach_name(self.meta.object_name.clone()))?,
            };

        let found = self.data.len();
        let expected = *num as usize * self.meta.captured_attribute.elem_count();
        if expected != found {
            Err(crate::AttributeError::LengthMismatch { found, expected }
                .attach_attr(self.meta.captured_attribute)
                .attach_name(self.meta.object_name.clone()))?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum OwnedInputBulk {
    Bool(Vec<bool>),
    Floats(Vec<f32>),
    Ints(Vec<i32>),
}

impl From<InputBulk<'_>> for OwnedInputBulk {
    fn from(value: InputBulk) -> Self {
        match value {
            InputBulk::Bool(cow) => Self::Bool(cow.into_owned()),
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

const BOOL: &str = "bool";
const FLOAT: &str = "float";
const INT: &str = "int";

impl OwnedInputBulk {
    #[inline]
    pub fn assume_bools(&self) -> Result<&[bool], AttributeError> {
        let expected = BOOL;
        let found = match self {
            Self::Bool(vec) => return Ok(vec.as_slice()),
            Self::Floats(_) => FLOAT,
            Self::Ints(_) => INT,
        };
        Err(AttributeError::TypeMismatch { expected, found })
    }

    #[inline]
    pub fn assume_floats<T: bytemuck::Pod>(&self) -> Result<&[T], AttributeError> {
        let expected = FLOAT;
        let found = match self {
            Self::Bool(_) => BOOL,
            Self::Floats(vec) => return Ok(bytemuck::try_cast_slice(vec.as_slice())?),
            Self::Ints(_) => INT,
        };
        Err(AttributeError::TypeMismatch { expected, found })
    }

    #[inline]
    pub fn assume_ints<T: bytemuck::Pod>(&self) -> Result<&[T], AttributeError> {
        let expected = INT;
        let found = match self {
            Self::Bool(_) => BOOL,
            Self::Floats(_) => FLOAT,
            Self::Ints(vec) => return Ok(bytemuck::try_cast_slice(vec.as_slice())?),
        };
        Err(AttributeError::TypeMismatch { expected, found })
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
        bulk.append(&mut &mut random_particle_bulk(
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
