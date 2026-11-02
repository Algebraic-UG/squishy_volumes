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

use iter_enumeration::IntoIterEnum2;
use std::{iter::once, num::NonZeroU64};

use squishy_volumes_mesh_util::{BoundingVolumeHierarchy, Node};

use crate::{Allocation, AllowedInBinding, GpuAllocatorError, prefix_sum_on_cpu};

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct BoundingVolumeHierarchyMeta {
    level: u32,
    leaf_size: f32,
    offset_x: i32,
    offset_y: i32,
    offset_z: i32,
}

impl AllowedInBinding for BoundingVolumeHierarchyMeta {
    const ALIGNMENT: NonZeroU64 = u32::ALIGNMENT;
}

#[derive(Clone)]
pub struct BoundingVolumeHierarchyAllocations {
    pub meta: Allocation,
    pub nodes: Allocation,
    pub indices: Allocation,
}

impl BoundingVolumeHierarchyAllocations {
    pub fn new(
        device: &wgpu::Device,
        leaf_size: f32,
        bvh: &BoundingVolumeHierarchy,
    ) -> Result<Self, GpuAllocatorError> {
        let offset = bvh.aabb().min;
        let meta = Allocation::new(
            device,
            "meta",
            &[BoundingVolumeHierarchyMeta {
                level: bvh.level(),
                leaf_size,
                offset_x: offset.x,
                offset_y: offset.y,
                offset_z: offset.z,
            }],
        )?;

        let indices_counts: Vec<u32> = bvh
            .nodes()
            .iter()
            .map(|node| {
                if let Node::Leaf(leaf) = node {
                    leaf.indices().len() as u32
                } else {
                    0
                }
            })
            .collect();
        let starts = prefix_sum_on_cpu(&indices_counts);
        let end = starts.last().expect("no starts") + indices_counts.last().expect("no counts");
        let start_and_ends: Vec<(u32, u32)> = starts
            .iter()
            .zip(starts.iter().skip(1).chain(once(&end)))
            .map(|(&start, &end)| (start, end))
            .collect();

        let node_sizes: Vec<u32> = bvh
            .nodes()
            .iter()
            .map(|node| {
                if let Node::Internal(internal) = node {
                    2 + internal
                        .children()
                        .iter()
                        .filter(|child| child.is_some())
                        .count() as u32
                } else {
                    4
                }
            })
            .collect();
        let node_indices = prefix_sum_on_cpu(&node_sizes);

        let nodes: Vec<u32> = bvh
            .nodes()
            .iter()
            .enumerate()
            .flat_map(|(i, node)| match node {
                Node::Internal(internal) => {
                    let mut result = vec![0, 0];
                    for child in 0..32 {
                        let Some(old_index) = internal.children()[child] else {
                            continue;
                        };
                        result[0] |= 1 << child;
                        result.push(node_indices[old_index as usize]);
                    }
                    for child in 0..32 {
                        let Some(old_index) = internal.children()[32 + child] else {
                            continue;
                        };
                        result[1] |= 1 << child;
                        result.push(node_indices[old_index as usize]);
                    }
                    result.into_iter().iter_enum_2a()
                }
                Node::Leaf(_) => {
                    [
                        0, // no children
                        0, // no children
                        start_and_ends[i].0,
                        start_and_ends[i].1,
                    ]
                    .into_iter()
                    .iter_enum_2b()
                }
            })
            .collect();
        let nodes = Allocation::new(device, "nodes", &nodes)?;

        let indices: Vec<u32> = bvh
            .nodes()
            .iter()
            .filter_map(|node| {
                if let Node::Leaf(leaf) = node {
                    Some(leaf)
                } else {
                    None
                }
            })
            .flat_map(|leaf| leaf.indices().iter().cloned())
            .collect();
        let indices = Allocation::new(device, "indices", &indices)?;

        Ok(Self {
            meta,
            nodes,
            indices,
        })
    }
}
