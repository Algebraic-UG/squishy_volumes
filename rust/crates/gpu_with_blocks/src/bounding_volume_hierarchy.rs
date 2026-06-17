// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use iter_enumeration::IntoIterEnum2;
use std::{iter::once, num::NonZeroU64};

use squishy_volumes_util::{BoundingVolumeHierarchy, bounding_volume_hierarchy::Node};

use crate::{Allocation, AllowedInBinding, prefix_sum_on_cpu};

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
    pub fn new(device: &wgpu::Device, leaf_size: f32, bvh: &BoundingVolumeHierarchy) -> Self {
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
        );

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
        let nodes = Allocation::new(device, "nodes", &nodes);

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
        let indices = Allocation::new(device, "indices", &indices);

        Self {
            meta,
            nodes,
            indices,
        }
    }
}
