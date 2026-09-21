// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::{InputObjectCollider, InputObjectParticles};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub struct FrameBulk {
    pub meta: FrameBulkMeta,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub struct FrameBulkMeta {
    pub object_name: String,
    pub captured_attribute: BulkAttribute,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum BulkAttribute {
    Particles(FrameBulkParticles),
    Collider(FrameBulkCollider),
}

impl BulkAttribute {
    fn elem_size(&self) -> usize {
        match self {
            Self::Particles(inner) => inner.elem_size(),
            Self::Collider(inner) => inner.elem_size(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkParticles {
    Flags,
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
    fn elem_size(&self) -> usize {
        match self {
            Self::Flags => size_of::<u32>(),
            Self::ColliderBits => size_of::<u32>(),
            Self::Transforms => size_of::<[[f32; 4]; 4]>(),
            Self::Sizes => size_of::<f32>(),
            Self::Densities => size_of::<f32>(),
            Self::YoungsModuluses => size_of::<f32>(),
            Self::PoissonsRatios => size_of::<f32>(),
            Self::InitialPositions => size_of::<[f32; 3]>(),
            Self::InitialVelocity => size_of::<[f32; 3]>(),
            Self::ViscosityDynamic => size_of::<f32>(),
            Self::ViscosityBulk => size_of::<f32>(),
            Self::Exponent => size_of::<u32>(),
            Self::BulkModulus => size_of::<f32>(),
            Self::SandAlpha => size_of::<f32>(),
            Self::GoalPositions => size_of::<[f32; 3]>(),
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, PartialOrd)]
pub enum FrameBulkCollider {
    VertexPositions,
    Triangles,
    TriangleFrictions,
    TriangleDampings,
}

impl FrameBulkCollider {
    fn elem_size(&self) -> usize {
        match self {
            FrameBulkCollider::VertexPositions => size_of::<[f32; 3]>(),
            FrameBulkCollider::Triangles => size_of::<[u32; 3]>(),
            FrameBulkCollider::TriangleFrictions => size_of::<f32>(),
            FrameBulkCollider::TriangleDampings => size_of::<f32>(),
        }
    }
}

#[cfg(test)]
pub fn random_particle_bulk(
    object_name: String,
    num_particles: u32,
    rng: &mut impl rand::Rng,
) -> Vec<FrameBulk> {
    use rand::RngExt as _;
    vec![FrameBulk {
        meta: FrameBulkMeta {
            object_name,
            captured_attribute: BulkAttribute::Particles(FrameBulkParticles::Flags),
        },
        data: bytemuck::cast_slice(
            &rng.random_iter::<u32>()
                .take(num_particles as usize)
                .collect::<Vec<_>>(),
        )
        .to_vec(),
    }]
}

#[cfg(test)]
pub fn random_collider_bulk(
    object_name: String,
    num_vertices: u32,
    num_triangles: u32,
    rng: &mut impl rand::Rng,
) -> Vec<FrameBulk> {
    use rand::RngExt as _;
    vec![
        FrameBulk {
            meta: FrameBulkMeta {
                object_name: object_name.clone(),
                captured_attribute: BulkAttribute::Collider(FrameBulkCollider::VertexPositions),
            },
            data: bytemuck::cast_slice(
                &rng.random_iter::<[f32; 3]>()
                    .take(num_vertices as usize)
                    .collect::<Vec<_>>(),
            )
            .to_vec(),
        },
        FrameBulk {
            meta: FrameBulkMeta {
                object_name: object_name.clone(),
                captured_attribute: BulkAttribute::Collider(FrameBulkCollider::Triangles),
            },
            data: bytemuck::cast_slice(
                &rng.random_iter::<[u32; 3]>()
                    .take(num_triangles as usize)
                    .collect::<Vec<_>>(),
            )
            .to_vec(),
        },
    ]
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputFrame {
    pub animated_globals: squishy_volumes_util::AnimatedGlobals,
    pub bulk: Vec<FrameBulk>,
}

impl InputFrame {
    pub fn verify(&self, header: &crate::InputHeader) -> Result<(), crate::FrameVerifcationError> {
        for FrameBulk { meta, data } in &self.bulk {
            let header_obj = header.objects.get(&meta.object_name).ok_or(
                crate::ObjectError::ObjectNotInHeader {
                    name: meta.object_name.clone(),
                },
            )?;

            let num = match (header_obj, &meta.captured_attribute) {
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
                _ => Err(crate::ObjectError::ObjectChangedType {
                    name: meta.object_name.clone(),
                })?,
            };

            let found = data.len();
            let expected = *num as usize * meta.captured_attribute.elem_size();
            if expected != found {
                Err(crate::FrameVerifcationError::LengthMismatch {
                    name: meta.object_name.clone(),
                    attribute: format!("{:?}", meta.captured_attribute),
                    found,
                    expected,
                })?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
impl InputFrame {
    pub fn test_input_0(num_particles: u32, num_vertices: u32, num_triangles: u32) -> Self {
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
        Self {
            animated_globals: squishy_volumes_util::AnimatedGlobals {
                gravity_x: 1.,
                gravity_y: 2.,
                gravity_z: 3.,
                goal_stiffness: 42.,
                goal_damping: 0.5,
                damping: 1.23,
            },
            bulk,
        }
    }

    pub fn test_input_1(num_particles: u32) -> Self {
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

        Self {
            animated_globals: squishy_volumes_util::AnimatedGlobals {
                gravity_x: 2.,
                gravity_y: 3.,
                gravity_z: 4.,
                goal_stiffness: 43.,
                goal_damping: 0.8,
                damping: 1.2,
            },
            bulk,
        }
    }

    pub fn test_input_2() -> Self {
        Self {
            animated_globals: squishy_volumes_util::AnimatedGlobals {
                gravity_x: 3.,
                gravity_y: 4.,
                gravity_z: 5.,
                goal_stiffness: 44.,
                goal_damping: 0.1,
                damping: 0.,
            },
            bulk: Default::default(),
        }
    }
}
