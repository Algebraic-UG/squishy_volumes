// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

mod allocate_blocks;
mod build_cells;
mod build_hash_table;
mod build_hash_table_colors;
mod cells_to_colorkeys;
mod cells_to_murmur;
mod color_cells;
mod context;
mod count_subkeys;
mod download;
mod error;
mod find_cell_boundaries;
mod offsets_to_indirect;
mod permute_cells;
mod permute_positions;
mod pipeline_part;
mod positions_to_keys;
mod prefix_sum;
mod prepare_grid;
mod radix_sort;
mod recycle_to_indirect;
mod reorder;
mod sort_positions_into_cells;
mod util;

pub use allocate_blocks::*;
pub use build_cells::*;
pub use build_hash_table::*;
pub use build_hash_table_colors::*;
pub use cells_to_colorkeys::*;
pub use cells_to_murmur::*;
pub use color_cells::*;
pub use context::*;
pub use count_subkeys::*;
pub use download::*;
pub use error::*;
pub use find_cell_boundaries::*;
pub use offsets_to_indirect::*;
pub use permute_cells::*;
pub use permute_positions::*;
pub use pipeline_part::*;
pub use positions_to_keys::*;
pub use prefix_sum::*;
pub use prepare_grid::*;
pub use radix_sort::*;
pub use recycle_to_indirect::*;
pub use reorder::*;
pub use sort_positions_into_cells::*;
pub use util::*;

#[cfg(test)]
mod test_util;
#[cfg(test)]
use test_util::*;
