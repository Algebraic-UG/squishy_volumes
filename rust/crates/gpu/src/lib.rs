// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub use wgpu;

mod allocator;
mod context;
mod download;
mod error;
mod pipeline_part;
mod util;

pub use allocator::*;
pub use context::*;
pub use download::*;
pub use error::*;
pub use pipeline_part::*;
pub use util::*;

//pub mod count_subkeys;
pub mod prefix_sum;
//pub mod radix_sort;
//pub mod reorder_indices;

//pub use count_subkeys::CountSubkeys;
pub use prefix_sum::PrefixSum;
//pub use radix_sort::RadixSort;
//pub use reorder_indices::ReorderIndices;

//mod allocate_blocks;
//mod build_cells;
//mod build_hash_table;
//mod build_hash_table_colors;
//mod cells_to_colorkeys;
//mod cells_to_murmur;
//mod color_cells;
//mod color_cells_2;
//mod find_cell_boundaries;
//mod offsets_to_indirect;
//mod permute_positions;
//mod positions_to_keys;
//mod prepare_grid;
//mod recycle_to_indirect;
//mod scatter_mass;
//mod reorder_particles;
//mod sort_positions_into_cells;

//pub use allocate_blocks::*;
//pub use build_cells::*;
//pub use build_hash_table::*;
//pub use build_hash_table_colors::*;
//pub use cells_to_colorkeys::*;
//pub use cells_to_murmur::*;
//pub use color_cells::*;
//pub use color_cells_2::*;
//pub use find_cell_boundaries::*;
//pub use offsets_to_indirectr:*;
//pub use permute_positions::*;
//pub use positions_to_keys::*;
//pub use prepare_grid::*;
//pub use recycle_to_indirect::*;
//pub use reorder::*;
//pub use scatter_mass::*;
//pub use reorder_particles::*;
//pub use sort_positions_into_cells::*;

#[cfg(test)]
mod test_util;
#[cfg(test)]
use test_util::*;
