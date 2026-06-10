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
    nodes: Vec<Node>,
}

impl BoundingVolumeHierarchy {
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
}

pub enum Node {
    Internal(Internal),
    Leaf(Leaf),
}

impl Node {
    pub fn aabb(&self) -> Aabb<Vector3<i32>> {
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
                let child_level = level - 1;
                let children = from_fn(|child| {
                    let child = child as i32;
                    #[rustfmt::skip]
                    #[allow(clippy::identity_op)]
                    let child_offset = aabb.min
                        + Vector3::new(
                            ((child >> 4) & 3) << (2 * child_level),
                            ((child >> 2) & 3) << (2 * child_level),
                            ((child >> 0) & 3) << (2 * child_level),
                        );
                    let child_aabb = aabb_from_offset_and_level(&child_offset, child_level);
                    let child_indices: Vec<u32> = indices
                        .iter()
                        .cloned()
                        .filter(|index| {
                            let to_check = aabbs[*index as usize];
                            to_check.has_overlap(&child_aabb)
                        })
                        .collect();

                    if child_indices.is_empty() {
                        None
                    } else {
                        Some(create(aabbs, nodes, child_level, child_aabb, child_indices))
                    }
                });
                nodes.push(Node::Internal(Internal { aabb, children }));
            }
            nodes.len() as u32 - 1
        }

        let mut nodes: Vec<Node> = Default::default();
        let indices = (0..aabbs.len() as u32).collect();
        create(&aabbs, &mut nodes, level, root_aabb, indices);

        Some(Self { level, nodes })
    }

    pub fn query(&self, point: &Vector3<i32>) -> &[u32] {
        // tree could be empty
        let Some(root) = self.nodes.last() else {
            return Default::default();
        };
        // query could be outside
        if !root.aabb().contains(point) {
            return Default::default();
        }
        // root could be leaf (maybe not? level is at least 1)
        let mut internal = match root {
            Node::Internal(internal) => internal,
            Node::Leaf(Leaf { indices, .. }) => {
                return indices;
            }
        };

        // the bits of the query encode the child 'path'
        let q = (point - internal.aabb.min).map(|c| c as u32);
        for level in (0..=self.level).rev() {
            #[rustfmt::skip]
            #[allow(clippy::identity_op)]
            let child =
                  (((q[0] >> (2 * level)) & 3) << 4)
                | (((q[1] >> (2 * level)) & 3) << 2)
                | (((q[2] >> (2 * level)) & 3) << 0);
            match internal.children[child as usize].map(|index| &self.nodes[index as usize]) {
                Some(Node::Internal(next_internal)) => {
                    internal = next_internal;
                }
                Some(Node::Leaf(Leaf { indices, .. })) => {
                    return indices;
                }
                // empty space
                None => {
                    return Default::default();
                }
            }
        }
        unreachable!();
    }
}
