// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_file_input::{
    AttributeError, BulkAttribute, FrameBulkCollider, FrameBulkParticles, InputConsts, InputFrame,
    InputObjectCollider, InputRangeCollider, InputRangeParticles, InputRanges, InputReader,
    OwnedFrameBulk,
};
use squishy_volumes_mesh_util::{Topology, TopologyInput};

use crate::Collider;

#[derive(thiserror::Error, Debug)]
pub enum FrameInputError {
    #[error("Wanted to interpolate from {frame_low}, but {frame} is loaded")]
    WrongFrameLoaded { frame_low: usize, frame: usize },

    #[error("Failed to input from file")]
    InputError(#[from] squishy_volumes_file_input::InputError),

    #[error("Something is wrong with the mesh inputs")]
    MeshError(#[from] squishy_volumes_mesh_util::Error),
}

pub struct FrameInput {
    frame: usize,
    input_reader: InputReader,
    input_ranges: InputRanges,

    topology: Topology,

    // needs to be rebuilt every frame change
    bvh: squishy_volumes_mesh_util::BoundingVolumeHierarchy,

    collider_start: Collider,
    collider_end: Collider,

    // from a to b (or zero)
    vertex_velocities: Vec<nalgebra::Vector3<f32>>,

    goal_positions_start: Vec<nalgebra::Vector3<f32>>,
    goal_positions_end: Vec<nalgebra::Vector3<f32>>,

    next_input_frame: Option<InputFrame>,
}

impl FrameInput {
    pub fn new(
        mut input_reader: InputReader,
        io_goal_positions: Vec<[f32; 3]>,
        io_collider: squishy_volumes_file_frame::Collider,
        frame: usize,
    ) -> Result<Self, FrameInputError> {
        let inv_scale = 1. / input_reader.header().consts.simulation_scale;
        let input_ranges = InputRanges::new(&input_reader.header().objects);

        let topology = create_topology(&mut input_reader)?;

        let collider_start: Collider = io_collider.into();

        let goal_positions_start = bytemuck::cast_vec(io_goal_positions);

        let next_input_frame = (frame + 1 < input_reader.len())
            .then(|| input_reader.read_frame(frame + 1))
            .transpose()?;

        let mut collider_end = collider_start.clone();
        let mut goal_positions_end = goal_positions_start.clone();
        let vertex_velocities;
        if let Some(next_input_frame) = next_input_frame.as_ref() {
            update_collider(
                inv_scale,
                &input_ranges,
                &mut collider_end,
                &next_input_frame.bulk,
            )
            .map_err(|error| error.attach_frame(frame + 1))?;
            vertex_velocities = linear_vertex_velocities(
                &input_reader.header().consts,
                &collider_start,
                &collider_end,
            );

            update_goal_positions(
                inv_scale,
                &input_ranges,
                &mut goal_positions_end,
                &next_input_frame.bulk,
            )
            .map_err(|error| error.attach_frame(frame + 1))?;
        } else {
            vertex_velocities =
                vec![nalgebra::Vector3::zeros(); collider_start.vertex_positions.len()];
        };

        let bvh = update_bvh(
            &input_reader.header().consts,
            &topology,
            &collider_start,
            &collider_end,
        );

        Ok(Self {
            frame,
            input_reader,
            input_ranges,
            topology,
            bvh,
            collider_start,
            collider_end,
            vertex_velocities,
            next_input_frame,
            goal_positions_start,
            goal_positions_end,
        })
    }

    pub fn frame(&self) -> usize {
        self.frame
    }

    pub fn next_input_frame(&mut self) -> Option<InputFrame> {
        self.next_input_frame.take()
    }

    pub fn load_next(&mut self) -> Result<(), FrameInputError> {
        self.frame += 1;

        if self.frame >= self.input_reader.len() {
            return Ok(());
        }

        self.collider_start = self.collider_end.clone();
        self.goal_positions_start = self.goal_positions_end.clone();

        self.next_input_frame = (self.frame + 1 < self.input_reader.len())
            .then(|| self.input_reader.read_frame(self.frame + 1))
            .transpose()?;

        let inv_scale = 1. / self.input_reader.header().consts.simulation_scale;
        if let Some(next_input_frame) = self.next_input_frame.as_ref() {
            update_collider(
                inv_scale,
                &self.input_ranges,
                &mut self.collider_end,
                &next_input_frame.bulk,
            )
            .map_err(|error| error.attach_frame(self.frame + 1))?;
            self.vertex_velocities =
                linear_vertex_velocities(self.consts(), &self.collider_start, &self.collider_end);

            update_goal_positions(
                inv_scale,
                &self.input_ranges,
                &mut self.goal_positions_end,
                &next_input_frame.bulk,
            )
            .map_err(|error| error.attach_frame(self.frame + 1))?;
        } else {
            self.vertex_velocities =
                vec![nalgebra::Vector3::zeros(); self.collider_start.vertex_positions.len()];
        };

        self.bvh = update_bvh(
            self.consts(),
            &self.topology,
            &self.collider_start,
            &self.collider_end,
        );

        Ok(())
    }

    pub fn consts(&self) -> &InputConsts {
        &self.input_reader.header().consts
    }

    pub fn input_ranges(&self) -> &InputRanges {
        &self.input_ranges
    }

    pub fn collider_start(&self) -> &Collider {
        &self.collider_start
    }

    pub fn collider_end(&self) -> &Collider {
        &self.collider_end
    }

    pub fn goal_positions_start(&self) -> &[nalgebra::Vector3<f32>] {
        &self.goal_positions_start
    }

    pub fn goal_positions_end(&self) -> &[nalgebra::Vector3<f32>] {
        &self.goal_positions_end
    }

    pub fn topology(&self) -> &squishy_volumes_mesh_util::Topology {
        &self.topology
    }

    pub fn bvh(&self) -> &squishy_volumes_mesh_util::BoundingVolumeHierarchy {
        &self.bvh
    }

    pub fn vertex_velocities(&self) -> &[nalgebra::Vector3<f32>] {
        &self.vertex_velocities
    }

    pub fn frame_factor(&self, time: f64) -> Result<f32, FrameInputError> {
        let frame_time = time * self.consts().frames_per_second as f64;
        let frame_low = frame_time.floor() as usize;

        if self.frame != frame_low {
            return Err(FrameInputError::WrongFrameLoaded {
                frame_low,
                frame: self.frame,
            });
        }

        Ok((frame_time % 1.) as f32)
    }
}

fn linear_vertex_velocities(
    consts: &InputConsts,
    collider_start: &Collider,
    collider_end: &Collider,
) -> Vec<nalgebra::Vector3<f32>> {
    collider_start
        .vertex_positions
        .iter()
        .zip(&collider_end.vertex_positions)
        .map(|(start, end)| (end - start) * consts.frames_per_second as f32)
        .collect()
}

fn update_bvh(
    consts: &InputConsts,
    topology: &squishy_volumes_mesh_util::Topology,
    collider_start: &Collider,
    collider_end: &Collider,
) -> squishy_volumes_mesh_util::BoundingVolumeHierarchy {
    use squishy_volumes_util::Aabb;

    let margin = consts.forget_distance();
    let aabbs = topology
        .triangle_indices()
        .iter()
        .map(|triangle| {
            let aabb = Aabb::new_from_ref(triangle.iter().flat_map(|vertex_index| {
                [
                    &collider_start.vertex_positions[*vertex_index as usize],
                    &collider_end.vertex_positions[*vertex_index as usize],
                ]
            }));

            Aabb {
                min: aabb
                    .min
                    .map(|c| ((c - margin) / consts.leaf_size).floor() as i32),
                max: aabb
                    .max
                    .map(|c| ((c + margin) / consts.leaf_size).ceil() as i32),
            }
        })
        .collect();

    squishy_volumes_mesh_util::BoundingVolumeHierarchy::new(aabbs, consts.leaf_threshold)
}

fn create_topology(input_reader: &mut InputReader) -> Result<Topology, FrameInputError> {
    let bulk = input_reader.read_frame(0)?.bulk;
    let mut topology_inputs = Vec::new();
    (|| -> Result<(), squishy_volumes_file_input::FrameVerifcationError> {
        for bulk in &bulk {
            if bulk.meta.captured_attribute != BulkAttribute::Collider(FrameBulkCollider::Triangles)
            {
                continue;
            }
            let InputObjectCollider {
                collider_id,
                num_vertices,
                ..
            } = input_reader
                .header()
                .get_collider_input_object(&bulk.meta.object_name)?;

            topology_inputs.push(TopologyInput {
                name: &bulk.meta.object_name,
                collider_id,
                num_vertices,
                triangle_indices: bulk.data.assume_ints().map_err(|error| {
                    error
                        .attach_attr(bulk.meta.captured_attribute)
                        .attach_name(bulk.meta.object_name.clone())
                })?,
            });
        }
        Ok(())
    })()
    .map_err(|error| error.attach_frame(0))?;
    Ok(Topology::new(topology_inputs.into_iter())?)
}

fn update_collider(
    inv_scale: f32,
    input_ranges: &InputRanges,
    collider: &mut Collider,
    bulk: &[OwnedFrameBulk],
) -> Result<(), squishy_volumes_file_input::FrameVerifcationError> {
    for bulk in bulk {
        if let BulkAttribute::Collider(ref attr) = bulk.meta.captured_attribute {
            let InputRangeCollider {
                vertex_range,
                triangle_range,
            } = input_ranges.get_collider_range(&bulk.meta.object_name)?;

            (|| -> Result<(), AttributeError> {
                match attr {
                    FrameBulkCollider::VertexPositions => {
                        // TODO: clean error for length mismatch
                        collider.vertex_positions[vertex_range.clone()]
                            .copy_from_slice(bulk.data.assume_floats()?);
                        collider.vertex_positions[vertex_range]
                            .iter_mut()
                            .for_each(|p| {
                                p[0] *= inv_scale;
                                p[1] *= inv_scale;
                                p[2] *= inv_scale;
                            });
                    }
                    FrameBulkCollider::TriangleFrictions => {
                        collider.triangle_frictions[triangle_range]
                            .copy_from_slice(bulk.data.assume_floats()?);
                    }
                    FrameBulkCollider::TriangleDampings => {
                        collider.triangle_dampings[triangle_range]
                            .copy_from_slice(bulk.data.assume_floats()?);
                    }
                    FrameBulkCollider::Triangles => {
                        // TODO:
                        tracing::error!(
                            "Updating the topology after frame 0 is not supported (yet?)"
                        );
                    }
                }
                Ok(())
            })()
            .map_err(|error| {
                error
                    .attach_attr(bulk.meta.captured_attribute)
                    .attach_name(bulk.meta.object_name.clone())
            })?;
        }
    }
    Ok(())
}

fn update_goal_positions(
    inv_scale: f32,
    input_ranges: &InputRanges,
    goal_positions: &mut [nalgebra::Vector3<f32>],
    bulk: &[OwnedFrameBulk],
) -> Result<(), squishy_volumes_file_input::FrameVerifcationError> {
    for bulk in bulk {
        if let BulkAttribute::Particles(ref attr) = bulk.meta.captured_attribute {
            let InputRangeParticles { particle_range } =
                input_ranges.get_particle_range(&bulk.meta.object_name)?;
            if let FrameBulkParticles::GoalPositions = attr {
                // TODO: clean error for length mismatch
                goal_positions[particle_range.clone()].copy_from_slice(
                    bulk.data.assume_floats().map_err(|error| {
                        error
                            .attach_attr(bulk.meta.captured_attribute)
                            .attach_name(bulk.meta.object_name.clone())
                    })?,
                );
                goal_positions[particle_range].iter_mut().for_each(|p| {
                    *p *= inv_scale;
                });
            }
        }
    }
    Ok(())
}
