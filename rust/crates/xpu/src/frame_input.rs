// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_file_input::InputError;

#[derive(thiserror::Error, Debug)]
pub enum FrameInputError {
    #[error("Wanted to interpolate from {frame_low}, but {frame} is loaded")]
    WrongFrameLoaded { frame_low: usize, frame: usize },

    #[error("Failed to input from file: {0}")]
    InputError(#[from] squishy_volumes_file_input::InputError),

    #[error("'{name}': length mismatch between '{attribute_a}' and '{attribute_b}'")]
    AttributeLengthMismatch {
        name: String,
        attribute_a: String,
        attribute_b: String,
    },

    #[error("Something is wrong with the mesh inputs: {0}")]
    MeshError(#[from] squishy_volumes_mesh_util::Error),

    #[error("Object error: {0}")]
    ObjectError(#[from] squishy_volumes_file_input::ObjectError),
}

pub struct FrameInput {
    frame: usize,

    consts: squishy_volumes_file_input::InputConsts,
    input_ranges: squishy_volumes_file_input::InputRanges,

    input_reader: squishy_volumes_file_input::InputReader,

    topology: squishy_volumes_mesh_util::Topology,

    // needs to be rebuilt every frame change
    bvh: squishy_volumes_mesh_util::BoundingVolumeHierarchy,

    // b could be none (end of input)
    a: InputInterpolationPoint,
    b: Option<InputInterpolationPoint>,
}

#[derive(Default)]
pub struct InputInterpolationPoint {
    frame: usize,

    gravity: nalgebra::Vector3<f32>,

    particle_flags: Vec<squishy_volumes_file_frame::ParticleFlags>,
    particle_goal_positions: Vec<nalgebra::Vector3<f32>>,

    vertex_positions: Vec<nalgebra::Vector3<f32>>,
    triangle_frictions: Vec<f32>,
}

impl InputInterpolationPoint {
    fn new(
        consts: &squishy_volumes_file_input::InputConsts,
        input_ranges: &squishy_volumes_file_input::InputRanges,
        frame: usize,
        squishy_volumes_file_input::InputFrame {
            gravity,
            particles_inputs,
            collider_inputs,
        }: squishy_volumes_file_input::InputFrame,
    ) -> Result<Self, FrameInputError> {
        let gravity = nalgebra::Vector3::from(gravity);

        let mut particle_flags: Vec<squishy_volumes_file_frame::ParticleFlags> =
            vec![Default::default(); input_ranges.total_particles];
        let mut particle_goal_positions: Vec<nalgebra::Vector3<f32>> =
            vec![Default::default(); input_ranges.total_particles];
        for (name, input) in particles_inputs.into_iter() {
            let particle_range = input_ranges.get_particle_range(&name)?;

            particle_flags.as_mut_slice()[particle_range.clone()]
                .copy_from_slice(bytemuck::cast_slice(&input.flags));

            if let Some(goal_positions) = input.goal_positions {
                if input.flags.len() != goal_positions.len() {
                    tracing::error!(
                        flags_len = input.flags.len(),
                        goal_positions_len = goal_positions.len()
                    );
                    return Err(FrameInputError::AttributeLengthMismatch {
                        name,
                        attribute_a: "Particle Flags".to_string(),
                        attribute_b: "Particle Goal Positions".to_string(),
                    });
                }

                particle_goal_positions.as_mut_slice()[particle_range.clone()]
                    .copy_from_slice(bytemuck::cast_slice(&goal_positions));
            }
        }

        let mut vertex_positions: Vec<nalgebra::Vector3<f32>> = Default::default();
        let mut triangle_frictions: Vec<f32> = Default::default();
        for mut input in collider_inputs.into_values() {
            vertex_positions.extend_from_slice(bytemuck::cast_slice(&input.vertex_positions));
            triangle_frictions.append(&mut input.triangle_frictions);
        }

        particle_goal_positions
            .iter_mut()
            .for_each(|p| *p /= consts.simulation_scale);
        vertex_positions
            .iter_mut()
            .for_each(|p| *p /= consts.simulation_scale);

        Ok(Self {
            frame,
            gravity,
            particle_flags,
            particle_goal_positions,
            vertex_positions,
            triangle_frictions,
        })
    }

    pub fn gravity(&self) -> &nalgebra::Vector3<f32> {
        &self.gravity
    }

    pub fn particle_flags(&self) -> &[squishy_volumes_file_frame::ParticleFlags] {
        &self.particle_flags
    }
    pub fn particle_goal_positions(&self) -> &[nalgebra::Vector3<f32>] {
        &self.particle_goal_positions
    }

    pub fn vertex_positions(&self) -> &[nalgebra::Vector3<f32>] {
        &self.vertex_positions
    }
    pub fn triangle_frictions(&self) -> &[f32] {
        &self.triangle_frictions
    }
}

impl FrameInput {
    pub fn new(
        mut input_reader: squishy_volumes_file_input::InputReader,
        frame: usize,
    ) -> Result<Self, FrameInputError> {
        let input_header = input_reader.read_header()?;
        let consts = input_header.consts;
        let input_ranges = squishy_volumes_file_input::InputRanges::new(&input_header.objects);

        let collider_inputs = input_reader.read_frame(0)?.collider_inputs;
        // This shouldn't happen, maybe if someone messed with the serialization?
        if collider_inputs.len() > 16 {
            Err(InputError::TooManyColliders)?
        }

        let topology =
            squishy_volumes_mesh_util::Topology::new(collider_inputs.iter().enumerate().map(
                |(collider, (name, collider_input))| squishy_volumes_mesh_util::TopologyInput {
                    name,
                    collider: collider as u32,
                    num_vertices: collider_input.vertex_positions.len() as u32,
                    triangle_indices: bytemuck::cast_slice(&collider_input.triangle_indices),
                },
            ))?;

        let mut a = Default::default();
        let mut b = Default::default();
        load_points(
            &mut input_reader,
            &mut a,
            &mut b,
            &consts,
            &input_ranges,
            frame,
        )?;
        let a = a.expect("a missing");

        let bvh = update_bvh(&consts, &topology, &a, b.as_ref());

        Ok(Self {
            frame,
            consts,
            input_ranges,
            input_reader,
            topology,
            bvh,
            a,
            b,
        })
    }

    pub fn frame(&self) -> usize {
        self.frame
    }

    pub fn load(&mut self, frame: usize) -> Result<(), FrameInputError> {
        let prior_frame = self.a.frame;

        // weird little dance s.t. the type of a can be non-option
        let mut a = Some(std::mem::take(&mut self.a));
        load_points(
            &mut self.input_reader,
            &mut a,
            &mut self.b,
            &self.consts,
            &self.input_ranges,
            frame,
        )?;
        self.a = a.expect("a missing");

        if prior_frame != self.a.frame {
            self.bvh = update_bvh(&self.consts, &self.topology, &self.a, self.b.as_ref());
        }

        self.frame = frame;

        Ok(())
    }

    pub fn consts(&self) -> &squishy_volumes_file_input::InputConsts {
        &self.consts
    }

    pub fn topology(&self) -> &squishy_volumes_mesh_util::Topology {
        &self.topology
    }

    pub fn bvh(&self) -> &squishy_volumes_mesh_util::BoundingVolumeHierarchy {
        &self.bvh
    }

    pub fn a(&self) -> &InputInterpolationPoint {
        &self.a
    }

    pub fn b(&self) -> Option<&InputInterpolationPoint> {
        self.b.as_ref()
    }

    pub fn frame_factor(&self, time: f64) -> Result<f32, FrameInputError> {
        let frame_time = time * self.consts.frames_per_second as f64;
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

// after this, a is always Some
fn load_points(
    input_reader: &mut squishy_volumes_file_input::InputReader,
    a: &mut Option<InputInterpolationPoint>,
    b: &mut Option<InputInterpolationPoint>,
    consts: &squishy_volumes_file_input::InputConsts,
    input_ranges: &squishy_volumes_file_input::InputRanges,
    frame: usize,
) -> Result<(), FrameInputError> {
    let max_frame = input_reader.len() - 1;

    // if we're too far, just use the last available and skip b
    let frame = frame.min(max_frame);

    // a already correct, so b must be as well (could be none)
    if a.as_ref().is_some_and(|point| point.frame == frame) {
        return Ok(());
    }

    // always load something into a
    // could be that we already have the next in b
    if let Some(b) = b.take()
        && b.frame == frame
    {
        *a = Some(b);
    } else {
        let input_frame = input_reader.read_frame(frame)?;
        *a = Some(InputInterpolationPoint::new(
            consts,
            input_ranges,
            frame,
            input_frame,
        )?);
    }

    // might skip b
    if frame == max_frame {
        return Ok(());
    }

    // load b
    let frame = frame + 1;
    let input_frame = input_reader.read_frame(frame)?;
    *b = Some(InputInterpolationPoint::new(
        consts,
        input_ranges,
        frame,
        input_frame,
    )?);

    Ok(())
}

fn update_bvh(
    consts: &squishy_volumes_file_input::InputConsts,
    topology: &squishy_volumes_mesh_util::Topology,
    a: &InputInterpolationPoint,
    b: Option<&InputInterpolationPoint>,
) -> squishy_volumes_mesh_util::BoundingVolumeHierarchy {
    use squishy_volumes_util::Aabb;

    let margin = consts.forget_distance();
    let aabbs = topology
        .triangle_indices()
        .iter()
        .map(|triangle| {
            let aabb = if let Some(b) = b {
                Aabb::new_from_ref(triangle.iter().flat_map(|vertex_index| {
                    [
                        &a.vertex_positions[*vertex_index as usize],
                        &b.vertex_positions[*vertex_index as usize],
                    ]
                }))
            } else {
                Aabb::new_from_ref(
                    triangle
                        .iter()
                        .map(|vertex_index| &a.vertex_positions[*vertex_index as usize]),
                )
            };

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
