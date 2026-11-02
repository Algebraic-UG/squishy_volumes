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

#![recursion_limit = "256"]

use wgebra as _;

pub use wgpu;
pub use wgpu_profiler;

mod allocator;
mod allowed_in_binding;
mod bounding_volume_hierarchy;
mod combined_with_collider_bits;
mod compiled_module;
mod context;
mod download;
mod error;
mod gpu_state;
mod indirect;
mod pipeline_part;
mod profiler_output;
mod status;
mod svd;
mod time_step_limits;
mod util;

pub use gpu_state::{GpuRunParameters, GpuState};

pub mod test_data;

pub use allocator::*;
pub use allowed_in_binding::*;
pub use bounding_volume_hierarchy::*;
pub use combined_with_collider_bits::*;
pub use compiled_module::*;
pub use context::*;
pub use download::*;
pub use error::*;
pub use indirect::*;
pub use pipeline_part::*;
pub use profiler_output::*;
pub use status::*;
pub use svd::*;
pub use util::*;

pub mod advance_time;
pub mod animate_mesh;
pub mod bits_to_pops;
pub mod build_hash_tables;
pub mod collect;
pub mod collide;
pub mod count_subkeys;
pub mod counts_indirect;
pub mod cull_particles;
pub mod elastic;
pub mod external_force;
pub mod find_minimum;
pub mod find_minimum_time_step_limits;
pub mod fluid;
pub mod kernels;
pub mod len_to_indirect;
pub mod limit_time_step;
pub mod limit_time_step_per_particle;
pub mod meld_grid;
pub mod node_ids_to_murmur;
pub mod partition_nodes;
pub mod prefix_sum;
pub mod prepare_grid;
pub mod prepare_tmp;
pub mod radix_sort;
pub mod register_contributors;
pub mod reorder_indices;
pub mod sand;
pub mod scatter;
pub mod step;
pub mod test_status;
pub mod test_svd;
pub mod update_flags;
pub mod viscosity;

pub use advance_time::AdvanceTime;
pub use animate_mesh::AnimateMesh;
pub use bits_to_pops::BitsToPops;
pub use build_hash_tables::BuildHashTables;
pub use collect::Collect;
pub use collide::Collide;
pub use count_subkeys::CountSubkeys;
pub use counts_indirect::CountsIndirect;
pub use cull_particles::CullParticles;
pub use external_force::ExternalForce;
pub use find_minimum::FindMinimum;
pub use find_minimum_time_step_limits::FindMinimumTimeStepLimits;
pub use kernels::Kernels;
pub use len_to_indirect::LenToIndirect;
pub use limit_time_step::LimitTimeStep;
pub use limit_time_step_per_particle::LimitTimeStepPerParticle;
pub use meld_grid::MeldGrid;
pub use node_ids_to_murmur::NodeIdsToMurmur;
pub use partition_nodes::PartitionNodes;
pub use prefix_sum::PrefixSum;
pub use prepare_grid::PrepareGrid;
pub use prepare_tmp::PrepareTmp;
pub use radix_sort::RadixSort;
pub use register_contributors::RegisterContributors;
pub use reorder_indices::ReorderIndices;
pub use sand::Sand;
pub use scatter::Scatter;
pub use step::Step;
pub use test_status::TestStatus;
pub use test_svd::TestSvd;
pub use update_flags::UpdateFlags;
pub use viscosity::Viscosity;

#[cfg(test)]
mod cone;
#[cfg(test)]
mod test_util;
#[cfg(test)]
mod torus;
#[cfg(test)]
use test_util::*;
