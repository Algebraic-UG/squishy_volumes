// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::collections::BTreeMap;

use anyhow::{Context, Error, Result, ensure};
use blended_mpm_api::T;
use fxhash::FxHashMap;
use nalgebra::Vector3;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::math::NORMALIZATION_EPS;

use super::{
    Mesh, ObjectWithData, ScriptedFrame, SerializedVector,
    setup::{GlobalSettings, Object, Setup},
};

#[derive(Serialize, Deserialize)]
pub struct BulkData {
    pub serialized_vectors: BTreeMap<String, SerializedVector>,
}

#[derive(Serialize, Deserialize)]
pub struct SerializedSetup {
    pub settings: GlobalSettings,
    pub objects: Vec<ObjectWithHandles>,
    pub bulk_data: BulkData,
}

#[derive(Serialize, Deserialize)]
pub struct ObjectWithHandles {
    pub object: Object,
    pub mesh_handles: MeshHandles,
    pub scripted_handles: ScriptedHandles,
}

#[derive(Serialize, Deserialize)]
pub struct MeshHandles {
    pub vertices: String,
    pub triangles: String,
    pub triangle_normals: String,
}

#[derive(Serialize, Deserialize)]
pub struct ScriptedHandles {
    pub scripted_positions: String,
    pub scripted_orientations: String,
}

impl TryFrom<SerializedSetup> for Setup {
    type Error = Error;

    fn try_from(
        SerializedSetup {
            settings,
            objects,
            mut bulk_data,
        }: SerializedSetup,
    ) -> Result<Self> {
        let objects: Vec<ObjectWithData> = objects
            .into_iter()
            .map(|object_with_handles| -> Result<ObjectWithData> {
                let object = object_with_handles.object.clone();
                let (mesh, scripted_frames) = helper(&mut bulk_data, object_with_handles)
                    .with_context(|| {
                        format!("failed to decode mesh for object: {}", object.name)
                    })?;
                Ok(ObjectWithData {
                    object,
                    mesh,
                    scripted_frames,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self { settings, objects })
    }
}

fn helper(
    bulk_data: &mut BulkData,
    ObjectWithHandles {
        object,
        mesh_handles:
            MeshHandles {
                vertices,
                triangles,
                triangle_normals,
            },
        scripted_handles:
            ScriptedHandles {
                scripted_positions,
                scripted_orientations,
            },
    }: ObjectWithHandles,
) -> Result<(Mesh, Vec<ScriptedFrame>)> {
    info!("derializing {}", object.name);
    let mut get_data = |name: &str| {
        bulk_data
            .serialized_vectors
            .remove(name)
            .with_context(|| format!("missing bulk data: {name}"))
    };
    info!("parsing vertices");
    let mut vertices: Vec<Vector3<T>> = get_data(&vertices)?.try_into()?;
    info!("parsing triangles");
    let triangles: Vec<[u32; 3]> = get_data(&triangles)?.try_into()?;
    info!("parsing triangle normals");
    let mut triangle_normals: Vec<Option<Vector3<T>>> = get_data(&triangle_normals)?.try_into()?;

    info!("applying scale");
    vertices
        .iter_mut()
        .for_each(|v| v.component_mul_assign(&object.scale));

    info!("updating triangle normals");
    let fix_normal = |n: &mut Option<Vector3<T>>| {
        let s = object.scale;
        *n = n
            .map(|n| {
                Vector3::new(
                    if s.x != 0. { n.x / s.x } else { 0. },
                    if s.y != 0. { n.y / s.y } else { 0. },
                    if s.z != 0. { n.z / s.z } else { 0. },
                )
            })
            .and_then(|v| v.try_normalize(NORMALIZATION_EPS))
    };
    triangle_normals.iter_mut().for_each(fix_normal);

    let get_vertex_position = |i| {
        vertices
            .get(i)
            .cloned()
            .context("vertex index out of bounds")
    };
    let get_triangle = |i| triangles.get(i).context("triangle index out of bounds");
    let get_triangle_normal = |i| {
        triangle_normals
            .get(i)
            .cloned()
            .context("triangle normal index out of bounds")
    };

    info!("calculating vertex normals");
    let mut vertex_to_triangles: Vec<Vec<usize>> =
        (0..vertices.len()).map(|_| Default::default()).collect();
    for (triangle_idx, triangle) in triangles.iter().enumerate() {
        for vertex_idx in triangle {
            vertex_to_triangles
                .get_mut(*vertex_idx as usize)
                .context("vertex index out of bounds")?
                .push(triangle_idx);
        }
    }
    if vertex_to_triangles.iter().any(Vec::is_empty) {
        warn!("some vertices aren't part of any triangle");
    }
    let vertex_normals: Vec<Option<Vector3<T>>> = vertices
        .iter()
        .zip(vertex_to_triangles.into_iter())
        .enumerate()
        .map(
            |(vertex_idx, (position, containing_triangles))| -> Result<Option<Vector3<T>>> {
                Ok(containing_triangles
                    .into_iter()
                    .try_fold(
                        Vector3::zeros(),
                        |vertex_normal, triangle_idx| -> Result<Vector3<T>> {
                            let Some(triangle_normal) = get_triangle_normal(triangle_idx)? else {
                                return Ok(vertex_normal);
                            };
                            let mut others = get_triangle(triangle_idx)?
                                .iter()
                                .map(|i| *i as usize)
                                .filter(|i| *i != vertex_idx);
                            let a = others
                                .next()
                                .expect("there is one other vertex in triangle");
                            let b = others
                                .next()
                                .expect("there are two other vertices in triangle");

                            let (Some(a), Some(b)) = (
                                (get_vertex_position(a)? - *position)
                                    .try_normalize(NORMALIZATION_EPS),
                                (get_vertex_position(b)? - *position)
                                    .try_normalize(NORMALIZATION_EPS),
                            ) else {
                                return Ok(vertex_normal);
                            };

                            let factor = a.dot(&b).acos();

                            Ok(vertex_normal + triangle_normal * factor)
                        },
                    )?
                    .try_normalize(NORMALIZATION_EPS))
            },
        )
        .collect::<Result<_>>()?;

    info!("calculating edges");
    let order_edge = |[a, b]: [u32; 2]| if a < b { [a, b] } else { [b, a] };
    let mut edges_with_generating_triangles: FxHashMap<[u32; 2], Vec<usize>> = Default::default();
    for (triangle_idx, [a, b, c]) in triangles.iter().cloned().enumerate() {
        for edge in [[a, b], [b, c], [c, a]].into_iter().map(order_edge) {
            edges_with_generating_triangles
                .entry(edge)
                .or_default()
                .push(triangle_idx);
        }
    }
    info!("calculating edge normals");
    let (edges, edge_normals): (Vec<[u32; 2]>, Vec<Option<Vector3<T>>>) =
        edges_with_generating_triangles
            .into_iter()
            .map(
                |(edge, triangles)| -> Result<([u32; 2], Option<Vector3<T>>)> {
                    let mut triangles = triangles.into_iter();
                    let a: Option<Vector3<T>> = triangles
                        .next()
                        .map(get_triangle_normal)
                        .transpose()?
                        .flatten();
                    let b: Option<Vector3<T>> = triangles
                        .next()
                        .map(get_triangle_normal)
                        .transpose()?
                        .flatten();

                    if triangles.next().is_some() {
                        warn!("non manifold geometry");
                        return Ok((edge, None));
                    }

                    Ok((
                        edge,
                        match (a, b) {
                            (None, None) => None,
                            (None, b) => b,
                            (a, None) => a,
                            (Some(a), Some(b)) => (a + b).try_normalize(NORMALIZATION_EPS),
                        },
                    ))
                },
            )
            .collect::<Result<_>>()?;

    let mesh = Mesh {
        vertices,
        vertex_normals,
        edges,
        edge_normals,
        triangles,
        triangle_normals,
    };

    info!("verifying mesh");
    mesh.verify(&object.name)?;

    info!(
        vertices = mesh.vertices.len(),
        vertex_normals = mesh.vertex_normals.len(),
        edges = mesh.edges.len(),
        edge_normals = mesh.edge_normals.len(),
        triangles = mesh.triangles.len(),
        triangle_normals = mesh.triangle_normals.len(),
        "mesh verified",
    );

    info!("parsing scripted motion");
    let scripted_positions: Vec<_> = get_data(&scripted_positions)?.try_into()?;
    let scripted_orientations: Vec<_> = get_data(&scripted_orientations)?.try_into()?;
    ensure!(scripted_positions.len() == scripted_orientations.len());

    let scripted_frames = scripted_positions
        .into_iter()
        .zip(scripted_orientations)
        .map(|(position, orientation)| ScriptedFrame {
            position,
            orientation,
        })
        .collect();

    Ok((mesh, scripted_frames))
}
