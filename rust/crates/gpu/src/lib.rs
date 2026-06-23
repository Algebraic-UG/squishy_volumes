// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
mod indirect;
mod pipeline_part;
mod profiler_output;
mod util;

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
pub use util::*;

pub mod animate_mesh;
pub mod bits_to_pops;
pub mod build_hash_tables;
pub mod collect;
pub mod collide;
pub mod count_subkeys;
pub mod counts_indirect;
pub mod elastic;
pub mod kernels;
pub mod len_to_indirect;
pub mod meld_grid;
pub mod node_ids_to_murmur;
pub mod particle_parameters;
pub mod partition_nodes;
pub mod prefix_sum;
pub mod prepare_grid;
pub mod prepare_tmp;
pub mod radix_sort;
pub mod register_contributors;
pub mod reorder_indices;
pub mod scatter;
pub mod step;

pub use animate_mesh::AnimateMesh;
pub use bits_to_pops::BitsToPops;
pub use build_hash_tables::BuildHashTables;
pub use collect::Collect;
pub use collide::Collide;
pub use count_subkeys::CountSubkeys;
pub use counts_indirect::CountsIndirect;
pub use kernels::Kernels;
pub use len_to_indirect::LenToIndirect;
pub use meld_grid::MeldGrid;
pub use node_ids_to_murmur::NodeIdsToMurmur;
pub use partition_nodes::PartitionNodes;
pub use prefix_sum::PrefixSum;
pub use prepare_grid::PrepareGrid;
pub use prepare_tmp::PrepareTmp;
pub use radix_sort::RadixSort;
pub use register_contributors::RegisterContributors;
pub use reorder_indices::ReorderIndices;
pub use scatter::Scatter;
pub use step::Step;

#[cfg(test)]
mod test_util;
#[cfg(test)]
mod torus;
#[cfg(test)]
use test_util::*;
