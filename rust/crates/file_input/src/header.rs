// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputConsts {
    grid_node_size: f32,
    pub leaf_size: f32,
    pub leaf_threshold: u32,
    pub max_num_particles: u32,
    pub simulation_scale: f32,
    pub frames_per_second: u32,
    pub domain_min: [f32; 3],
    pub domain_max: [f32; 3],
}

#[cfg(test)]
impl InputConsts {
    pub fn test_input() -> Self {
        Self {
            leaf_size: 1.,
            leaf_threshold: 16,
            max_num_particles: 10000000,
            grid_node_size: 0.5,
            simulation_scale: 1.,
            frames_per_second: 24,
            domain_min: [-100.; 3],
            domain_max: [100.; 3],
        }
    }
}

impl InputConsts {
    pub fn scaled_grid_node_size(&self) -> f32 {
        self.grid_node_size / self.simulation_scale
    }

    pub fn unscaled_grid_node_size(&self) -> f32 {
        self.grid_node_size
    }

    pub fn accept_distance(&self) -> f32 {
        self.scaled_grid_node_size() * 2.
    }

    pub fn forget_distance(&self) -> f32 {
        self.scaled_grid_node_size() * 2.2
    }

    pub fn seconds_per_frame(&self) -> f64 {
        1. / self.frames_per_second as f64
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum InputObject {
    Particles {
        num_particles: usize,
    },
    Collider {
        num_vertices: usize,
        num_triangles: usize,
    },
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct InputHeader {
    pub consts: InputConsts,
    pub objects: std::collections::BTreeMap<String, InputObject>,
}

impl InputHeader {
    pub fn total_particles(&self) -> usize {
        self.objects
            .values()
            .map(|object| {
                if let InputObject::Particles { num_particles } = object {
                    *num_particles
                } else {
                    0
                }
            })
            .sum()
    }
}

#[derive(Clone, Debug)]
pub enum InputRange {
    Particles {
        particle_range: std::ops::Range<usize>,
    },
    Collider {
        vertex_range: std::ops::Range<usize>,
        triangle_range: std::ops::Range<usize>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct InputRanges {
    pub total_particles: usize,
    pub total_vertices: usize,
    pub total_triangles: usize,
    pub objects: std::collections::BTreeMap<String, InputRange>,
}

impl InputRanges {
    pub fn new(objects: &std::collections::BTreeMap<String, InputObject>) -> Self {
        objects
            .iter()
            .fold(Self::default(), |mut result, (name, object)| {
                let range = match object {
                    InputObject::Particles { num_particles } => {
                        result.total_particles += num_particles;
                        InputRange::Particles {
                            particle_range: result.total_particles - num_particles
                                ..result.total_particles,
                        }
                    }
                    InputObject::Collider {
                        num_vertices,
                        num_triangles,
                    } => {
                        result.total_vertices += num_vertices;
                        result.total_triangles += num_triangles;
                        InputRange::Collider {
                            vertex_range: result.total_vertices - num_vertices
                                ..result.total_vertices,
                            triangle_range: result.total_triangles - num_triangles
                                ..result.total_triangles,
                        }
                    }
                };
                result.objects.insert(name.clone(), range);
                result
            })
    }

    pub fn get_particle_range(
        &self,
        name: &str,
    ) -> Result<std::ops::Range<usize>, crate::ObjectError> {
        if let InputRange::Particles { particle_range } =
            self.objects
                .get(name)
                .ok_or(crate::ObjectError::ObjectNotInHeader {
                    name: name.to_string(),
                })?
        {
            Ok(particle_range.clone())
        } else {
            Err(crate::ObjectError::ObjectChangedType {
                name: name.to_string(),
            })
        }
    }
}
