// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub use wgpu;

mod allocator;
mod allowed_in_binding;
mod compiled_module;
mod context;
mod download;
mod error;
mod indirect;
mod pipeline_part;
mod triangle;
mod util;

pub use allocator::*;
pub use allowed_in_binding::*;
pub use compiled_module::*;
pub use context::*;
pub use download::*;
pub use error::*;
pub use indirect::*;
pub use pipeline_part::*;
pub use triangle::*;
pub use util::*;

pub mod allocate_blocks;
pub mod build_cells;
pub mod build_hash_table_from_cells;
pub mod cells_to_colorkeys;
pub mod cells_to_murmur;
pub mod collect;
pub mod color_cells;
//pub mod count_colliders;
//pub mod build_blocks;
pub mod count_subkeys;
pub mod counts_indirect;
pub mod elastic;
pub mod find_cell_boundaries;
pub mod kernels;
pub mod offsets_to_indirect;
pub mod particle_parameters;
pub mod permute_particles;
pub mod positions_to_keys;
pub mod prefix_sum;
pub mod prepare_grid;
pub mod radix_sort;
pub mod recycle_to_indirect;
pub mod reorder_indices;
pub mod scatter;
pub mod sort_positions_into_cells;
pub mod step;
pub mod triangle_sdf;

pub use allocate_blocks::AllocateBlocks;
pub use build_cells::BuildCells;
pub use build_hash_table_from_cells::BuildHashTableFromCells;
pub use cells_to_colorkeys::CellsToColorkeys;
pub use cells_to_murmur::CellsToMurmur;
pub use collect::Collect;
pub use color_cells::ColorCells;
pub use count_subkeys::CountSubkeys;
pub use counts_indirect::CountsIndirect;
pub use find_cell_boundaries::FindCellBoundaries;
pub use kernels::Kernels;
pub use offsets_to_indirect::OffsetsToIndirect;
pub use permute_particles::PermuteParticles;
pub use positions_to_keys::PositionsToKeys;
pub use prefix_sum::PrefixSum;
pub use prepare_grid::PrepareGrid;
pub use radix_sort::RadixSort;
pub use recycle_to_indirect::RecycleToIndirect;
pub use reorder_indices::ReorderIndices;
pub use scatter::Scatter;
pub use sort_positions_into_cells::SortPositionsIntoCells;
pub use step::Step;
pub use triangle_sdf::TriangleSdf;

#[cfg(test)]
mod test_util;
#[cfg(test)]
mod torus;
#[cfg(test)]
use test_util::*;
