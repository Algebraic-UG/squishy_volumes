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

use crate::{Aabb, aabb::AabbVector as _, triangle::Triangle};

#[derive(Default)]
pub struct BoundingVolumeHierarchy {
    level: u32,
    nodes: Vec<Node>,
}

impl BoundingVolumeHierarchy {
    pub fn level(&self) -> u32 {
        self.level
    }
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn aabb(&self) -> Aabb<Vector3<i32>> {
        self.nodes.first().expect("root missing").aabb()
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(Debug)]
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

#[derive(Debug)]
pub struct Internal {
    aabb: Aabb<Vector3<i32>>,
    children: [Option<u32>; NUM_CHILDREN],
}

impl Internal {
    pub fn children(&self) -> &[Option<u32>; NUM_CHILDREN] {
        &self.children
    }
}

#[derive(Debug)]
pub struct Leaf {
    aabb: Aabb<Vector3<i32>>,
    indices: Vec<u32>,
}

impl Leaf {
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}

impl BoundingVolumeHierarchy {
    pub fn new(aabbs: Vec<Aabb<Vector3<i32>>>, leaf_threshold: u32) -> Self {
        let aabb = aabbs
            .clone()
            .into_par_iter()
            .reduce(Aabb::default, |a, b| a.extend(&b.min).extend(&b.max));
        let longest_side = aabb.extents().max();

        if longest_side <= 0 {
            tracing::warn!("Bounding Volumes Hierarchy emtpy.");
            return Self::default();
        }

        let level = longest_side.ilog(4) + 1;
        let middle_2 = aabb.min + aabb.max;
        let offset = (middle_2 - Vector3::repeat(4i32.pow(level))) / 2;

        let root_aabb = aabb_from_offset_and_level(&offset, level);
        assert!(root_aabb.min.leq(&aabb.min));
        assert!(aabb.max.leq(&root_aabb.max));

        fn create(
            leaf_threshold: u32,
            aabbs: &[Aabb<Vector3<i32>>],
            nodes: &mut Vec<Node>,
            level: u32,
            aabb: Aabb<Vector3<i32>>,
            indices: Vec<u32>,
        ) -> u32 {
            let index = nodes.len() as u32;
            if level == 0 || indices.len() < leaf_threshold as usize {
                nodes.push(Node::Leaf(Leaf { aabb, indices }));
            } else {
                nodes.push(Node::Internal(Internal {
                    aabb,
                    children: [None; NUM_CHILDREN],
                }));
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
                        Some(create(
                            leaf_threshold,
                            aabbs,
                            nodes,
                            child_level,
                            child_aabb,
                            child_indices,
                        ))
                    }
                });
                let Node::Internal(internal) = &mut nodes[index as usize] else {
                    unreachable!();
                };
                internal.children = children;
            }
            index
        }

        let mut nodes: Vec<Node> = Default::default();
        let indices = (0..aabbs.len() as u32).collect();
        create(
            leaf_threshold,
            &aabbs,
            &mut nodes,
            level,
            root_aabb,
            indices,
        );

        Self { level, nodes }
    }

    pub fn query(&self, point: &Vector3<i32>) -> &[u32] {
        // tree could be empty
        let Some(root) = self.nodes.first() else {
            return Default::default();
        };
        // query could be outside
        if !root.aabb().contains(point) {
            return Default::default();
        }
        // root could be leaf (due to threshold)
        let mut internal = match root {
            Node::Internal(internal) => internal,
            Node::Leaf(Leaf { indices, .. }) => {
                return indices;
            }
        };

        // the bits of the query encode the child 'path'
        let q = (point - internal.aabb.min).map(|c| c as u32);
        for level in (0..self.level).rev() {
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

pub fn triangles_to_leaf_aabbs(
    leaf_size: f32,
    margin: f32,
    vertices: &[Vector3<f32>],
    triangles: &[Triangle],
) -> Vec<Aabb<Vector3<i32>>> {
    triangles
        .iter()
        .map(|triangle| {
            let aabb = Aabb::new(triangle.into_iter().map(|i| vertices[i as usize].xyz()));
            Aabb {
                min: aabb.min.map(|c| ((c - margin) / leaf_size).floor() as i32),
                max: aabb.max.map(|c| ((c + margin) / leaf_size).ceil() as i32),
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use super::*;
    use rand::prelude::*;
    use rand::rngs::ChaCha8Rng;

    fn generate_triangles() -> (Vec<Vector3<f32>>, Vec<Triangle>) {
        let mut rng = ChaCha8Rng::seed_from_u64(420);
        let n = 1000;
        (
            (0..n)
                .flat_map(|_| {
                    let a = Vector3::new(
                        rng.random_range(-20.0..20.0),
                        rng.random_range(-20.0..20.0),
                        rng.random_range(-20.0..20.0),
                    );
                    let b = a + Vector3::new(
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                    );
                    let c = a + Vector3::new(
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                        rng.random_range(-1.0..1.0),
                    );
                    [a, b, c]
                })
                .collect(),
            (0..n)
                .map(|i| Triangle {
                    a: i * 3,
                    b: i * 3 + 1,
                    c: i * 3 + 2,
                })
                .collect(),
        )
    }

    #[test]
    fn queries() {
        let (vertices, triangles) = generate_triangles();
        let leaf_size = 1.;
        let margin = 1.;
        let leaf_aabbs = triangles_to_leaf_aabbs(leaf_size, margin, &vertices, &triangles);
        let bvh = BoundingVolumeHierarchy::new(leaf_aabbs.clone(), 4).unwrap();
        let aabbs: Vec<_> = leaf_aabbs
            .iter()
            .map(|Aabb { min, max }| Aabb {
                min: min.map(|c| c as f32 * leaf_size),
                max: max.map(|c| c as f32 * leaf_size),
            })
            .collect();

        let mut rng = ChaCha8Rng::seed_from_u64(666);
        for _ in 0..1000 {
            let p = Vector3::new(
                rng.random_range(-25.0..25.0),
                rng.random_range(-25.0..25.0),
                rng.random_range(-25.0..25.0),
            );
            let q = p.map(|c| (c / leaf_size).floor() as i32);
            let subset: Vec<_> = aabbs
                .iter()
                .enumerate()
                .filter_map(|(i, aabb)| aabb.contains(&p).then_some(i as u32))
                .collect();
            let superset: BTreeSet<_> = bvh.query(&q).iter().cloned().collect();
            for &i in &subset {
                if superset.contains(&i) {
                    continue;
                }
                println!("{subset:?}");
                println!("{superset:?}");
                println!("for point: {p:?}, aka. query {q:?}");
                println!(
                    "this wasn't returned: {i}, aka. {:?}, or {:?}",
                    aabbs[i as usize], leaf_aabbs[i as usize]
                );
                panic!();
            }
        }
    }

    //#[test]
    //fn print() {
    //    let (vertices, triangles) = generate_triangles();

    //    use std::io::Write;

    //    let file = std::fs::File::create("asdf.obj").unwrap();
    //    let mut writer = std::io::BufWriter::new(file);

    //    writeln!(writer, "# Wavefront OBJ").unwrap();

    //    for v in vertices {
    //        writeln!(writer, "v {} {} {}", v.x, v.y, v.z).unwrap();
    //    }

    //    for [a, b, c] in triangles {
    //        // OBJ indices are 1-based
    //        writeln!(writer, "f {} {} {}", a + 1, b + 1, c + 1).unwrap();
    //    }
    //}
}
