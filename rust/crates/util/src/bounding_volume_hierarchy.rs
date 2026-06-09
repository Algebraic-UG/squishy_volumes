// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::array::from_fn;

use nalgebra::Vector3;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{Aabb, aabb::AabbVector as _};

pub struct BoundingVolumeHierarchy {
    level: u32,
    offset: Vector3<i32>,
    nodes: Vec<Node>,
    aabbs: Vec<Aabb<Vector3<i32>>>,
}

pub enum Node {
    Internal(Internal),
    Leaf(Leaf),
}

impl Node {
    fn aabb(&self) -> Aabb<Vector3<i32>> {
        match self {
            Node::Internal(internal) => internal.aabb,
            Node::Leaf(leaf) => leaf.aabb,
        }
    }
}

fn aabb_from_offset_and_level(offset: &Vector3<i32>, level: u32) -> Aabb<Vector3<i32>> {
    Aabb {
        min: *offset,
        max: offset + Vector3::repeat(4i32.pow(level)),
    }
}

const NUM_CHILDREN: usize = 64;
const LEAF_THRESHOLD: usize = 16;

pub struct Internal {
    aabb: Aabb<Vector3<i32>>,
    children: [Option<u32>; NUM_CHILDREN],
}

pub struct Leaf {
    aabb: Aabb<Vector3<i32>>,
    indices: Vec<u32>,
}

impl BoundingVolumeHierarchy {
    pub fn new(aabbs: Vec<Aabb<Vector3<i32>>>) -> Option<Self> {
        let aabb = aabbs
            .clone()
            .into_par_iter()
            .reduce(Aabb::default, |a, b| a.extend(b.min).extend(b.max));
        let longest_side = aabb.extents().max();

        if longest_side <= 0 {
            tracing::warn!("Bounding Volumes Hierarchy emtpy.");
            return None;
        }

        let level = longest_side.ilog(4) + 1;
        let middle_2 = aabb.min + aabb.max;
        let offset = (middle_2 - Vector3::repeat(4i32.pow(level))) / 2;

        let root_aabb = aabb_from_offset_and_level(&offset, level);
        assert!(root_aabb.min.leq(&aabb.min));
        assert!(aabb.max.leq(&root_aabb.max));

        fn create(
            aabbs: &[Aabb<Vector3<i32>>],
            nodes: &mut Vec<Node>,
            level: u32,
            aabb: Aabb<Vector3<i32>>,
            indices: Vec<u32>,
        ) -> u32 {
            if level == 0 || indices.len() < LEAF_THRESHOLD {
                nodes.push(Node::Leaf(Leaf { aabb, indices }));
            } else {
                let children = from_fn(|child| {
                    let child = child as i32;
                    let child_offset = aabb.min
                        + Vector3::new(
                            (child & 0x30) << (level * 4),
                            (child & 0x0C) << (level * 4),
                            (child & 0x03) << (level * 4),
                        );
                    let child_aabb = aabb_from_offset_and_level(&child_offset, level - 1);
                    let child_indices: Vec<u32> = indices
                        .iter()
                        .cloned()
                        .filter(|index| aabbs[*index as usize].has_overlap(&child_aabb))
                        .collect();

                    if child_indices.is_empty() {
                        None
                    } else {
                        Some(create(aabbs, nodes, level - 1, child_aabb, child_indices))
                    }
                });
                nodes.push(Node::Internal(Internal { aabb, children }));
            }
            nodes.len() as u32 - 1
        }

        let mut nodes: Vec<Node> = Default::default();
        let indices = (0..aabbs.len() as u32).collect();
        create(&aabbs, &mut nodes, level, root_aabb, indices);

        /*
        let point = aabbs[0].min;
        let query: Vector3<u32> = (point - offset).map(|c| (c / leaf_size).floor().max(0.) as u32);
        let mask = 0x0000000F << (4 * level);
        let child = ((query[0] & mask) << 8) | ((query[1] & mask) << 4) | (query[2] & mask);
        */

        Some(Self {
            level,
            offset,
            nodes,
            aabbs,
        })
    }
}
