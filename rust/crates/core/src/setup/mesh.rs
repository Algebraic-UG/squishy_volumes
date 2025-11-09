// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    iter::empty,
    num::NonZero,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use iter_enumeration::{IntoIterEnum2, IntoIterEnum3};
use nalgebra::{Matrix3, Vector2, Vector3};
use rayon::iter::{IntoParallelRefIterator, ParallelBridge, ParallelIterator};
use serde::{Deserialize, Serialize};
use squishy_volumes_api::T;
use thiserror::Error;
use tracing::{info, warn};

use crate::{Report, ReportInfo, report::REPORT_STRIDE};
use crate::{
    ensure_err,
    math::{Aabb, basis_from_direction_3d},
};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Mesh {
    pub vertices: Vec<Vector3<T>>,
    pub vertex_normals: Vec<Option<Vector3<T>>>,

    pub edges: Vec<[u32; 2]>,
    pub edge_normals: Vec<Option<Vector3<T>>>,

    pub triangles: Vec<[u32; 3]>,
    pub triangle_normals: Vec<Option<Vector3<T>>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SurfaceSample {
    pub position: Vector3<T>,
    pub normal: Vector3<T>,
}

#[derive(Debug)]
pub enum MeshElement {
    VertexPosition,
    VertexNormal,
    EdgeIndices,
    EdgeNormal,
    TriangleIndices,
    TriangleNormal,
}

#[derive(Error, Debug)]
pub enum MeshError {
    #[error("Operation was cancelled")]
    Cancelled,
    #[error("The mesh is empty")]
    Empty,
    #[error("These should have the same count {0:?} and {1:?}")]
    CountMismatch(MeshElement, MeshElement),
    #[error("Input was NaN for {0:?}")]
    NaN(MeshElement),
    #[error("Index out of bounds for {0:?}")]
    OutOfBounds(MeshElement),
    #[error("The sampling spacing must be strictly postive")]
    SamplingSpacing,
}

impl Mesh {
    pub fn verify(&self, name: &str) -> Result<(), MeshError> {
        ensure_err!(
            self.vertices.len() == self.vertex_normals.len(),
            MeshError::CountMismatch(MeshElement::VertexPosition, MeshElement::VertexNormal)
        );
        ensure_err!(
            self.vertices
                .iter()
                .all(|v| v.iter().all(|x| x.is_finite())),
            MeshError::NaN(MeshElement::VertexPosition),
        );
        ensure_err!(
            self.vertex_normals
                .iter()
                .filter_map(|n| *n)
                .all(|v| v.iter().all(|x| x.is_finite())),
            MeshError::NaN(MeshElement::VertexNormal),
        );
        let invalid =
            self.vertex_normals.len() - self.vertex_normals.iter().filter_map(|n| *n).count();
        if invalid != 0 {
            warn!(name, invalid, "invalid {:?}", MeshElement::VertexNormal);
        }

        ensure_err!(
            self.edges.len() == self.edge_normals.len(),
            MeshError::CountMismatch(MeshElement::EdgeIndices, MeshElement::EdgeNormal)
        );
        ensure_err!(
            self.edge_normals
                .iter()
                .filter_map(|n| *n)
                .all(|v| v.iter().all(|x| x.is_finite())),
            MeshError::NaN(MeshElement::EdgeNormal)
        );
        ensure_err!(
            self.edges.iter().all(|indices| indices
                .iter()
                .all(|idx| (*idx as usize) < self.vertices.len())),
            MeshError::OutOfBounds(MeshElement::EdgeIndices)
        );
        let invalid = self.edge_normals.len() - self.edge_normals.iter().filter_map(|n| *n).count();
        if invalid != 0 {
            warn!(name, invalid, "invalid edge normals");
        }

        ensure_err!(
            self.triangles.len() == self.triangle_normals.len(),
            MeshError::CountMismatch(MeshElement::TriangleIndices, MeshElement::TriangleNormal)
        );
        ensure_err!(
            self.triangles.iter().all(|indices| indices
                .iter()
                .all(|idx| (*idx as usize) < self.vertices.len())),
            MeshError::OutOfBounds(MeshElement::TriangleIndices)
        );
        ensure_err!(
            self.triangle_normals
                .iter()
                .filter_map(|n| *n)
                .all(|v| v.iter().all(|x| x.is_finite())),
            MeshError::NaN(MeshElement::TriangleNormal)
        );
        let invalid =
            self.triangle_normals.len() - self.triangle_normals.iter().filter_map(|n| *n).count();
        if invalid != 0 {
            warn!(name, invalid, "invalid triangle normals");
        }

        Ok(())
    }

    pub fn sample_surface(
        &self,
        run: Arc<AtomicBool>,
        spacing: T,
    ) -> Result<Vec<SurfaceSample>, MeshError> {
        ensure_err!(spacing > 0., MeshError::SamplingSpacing);

        let vertex_samples = self
            .vertices
            .iter()
            .zip(self.vertex_normals.iter())
            .filter_map(|(&position, normal)| {
                normal.map(|normal| SurfaceSample { position, normal })
            });

        let edge_samples =
            self.edges
                .iter()
                .zip(self.edge_normals.iter())
                .flat_map(move |([a, b], normal)| {
                    let Some(normal) = *normal else {
                        return empty().iter_enum_3a();
                    };

                    let a = self.vertices[*a as usize];
                    let b = self.vertices[*b as usize];
                    let ba = b - a;

                    let length = ba.norm();
                    if length == 0. {
                        return empty().iter_enum_3b();
                    }

                    let n = (length / spacing).max(1.) as u32;
                    (1..n)
                        .map(move |i| a + ba * (i as T / n as T))
                        .map(move |position| SurfaceSample { position, normal })
                        .iter_enum_3c()
                });

        let triangle_samples = self
            .triangles
            .iter()
            .zip(self.triangle_normals.iter())
            .flat_map(move |([a, b, c], normal)| {
                let Some(normal) = *normal else {
                    return empty().iter_enum_2a();
                };

                let a = self.vertices[*a as usize];
                let b = self.vertices[*b as usize];
                let c = self.vertices[*c as usize];

                let to_world = basis_from_direction_3d(normal);
                let to_local = to_world.transpose();

                let offset = (to_local * a).x;
                let a = (to_local * a).yz();
                let b = (to_local * b).yz();
                let c = (to_local * c).yz();

                let ab = a - b;
                let bc = b - c;
                let ca = c - a;

                let ab_n = ab.norm();
                let bc_n = bc.norm();
                let ca_n = ca.norm();

                let aabb = Aabb::new([a, b, c].into_iter());

                let extents = aabb.extents();
                let sx = extents.x;
                let sy = extents.y;

                let nx = (sx / spacing).max(1.) as u32;
                let ny = (sy / spacing).max(1.) as u32;

                // TODO: replace with lattice from Aabb?
                let candidates = (1..nx).flat_map(move |i| {
                    (1..ny).map(move |j| {
                        aabb.min
                            + extents
                                .component_mul(&Vector2::new(i as T / nx as T, j as T / ny as T))
                    })
                });

                candidates
                    .filter(move |p| {
                        let h_abp = -ab.perp(&(*p - a)) / ab_n * 2.;
                        let h_bcp = -bc.perp(&(*p - b)) / bc_n * 2.;
                        let h_cap = -ca.perp(&(*p - c)) / ca_n * 2.;

                        h_abp > spacing && h_bcp > spacing && h_cap > spacing
                    })
                    .map(move |sample| to_world * Vector3::new(offset, sample.x, sample.y))
                    .map(move |position| SurfaceSample { position, normal })
                    .iter_enum_2b()
            });

        empty()
            .chain(vertex_samples)
            .chain(edge_samples)
            .chain(triangle_samples)
            .par_bridge()
            .map(|sample| {
                ensure_err!(run.load(Ordering::Relaxed), MeshError::Cancelled);
                Ok(sample)
            })
            .collect::<Result<_, _>>()
    }

    pub fn sample_inside(
        &self,
        run: Arc<AtomicBool>,
        report: Report,
        spacing: T,
        randomness: T,
    ) -> Result<Vec<Vector3<T>>, MeshError> {
        ensure_err!(spacing > 0., MeshError::SamplingSpacing);

        let mut aabb = Aabb::new(self.vertices.iter().cloned());
        ensure_err!(
            aabb != Default::default() && !self.vertices.is_empty() && !self.triangles.is_empty(),
            MeshError::Empty
        );

        // hack to avoid perfect alignment with cubes
        aabb.min += aabb.extents() * 0.001;
        aabb.max -= aabb.extents() * 0.001;

        let to_positions = |[a, b, c]: &[u32; 3]| {
            [
                self.vertices[*a as usize],
                self.vertices[*b as usize],
                self.vertices[*c as usize],
            ]
        };

        let wind_triangle = move |position, triangle: &[u32; 3]| {
            let [a, b, c]: [Vector3<T>; 3] = to_positions(triangle).map(|x| x - position);

            let ab = a.dot(&b);
            let bc = b.dot(&c);
            let ca = c.dot(&a);

            let det_abc = Matrix3::from_columns(&[a, b, c]).determinant();

            let a = a.norm();
            let b = b.norm();
            let c = c.norm();

            let divisor = a * b * c + ab * c + bc * a + ca * b;

            det_abc.atan2(divisor) / std::f64::consts::TAU as T
        };

        #[derive(Debug)]
        struct Cell {
            winding: T,
            center: Vector3<T>,
            close_triangles: Vec<usize>,
        }

        let pre_compute_spacing = spacing * 10.;
        let close_measure = pre_compute_spacing * 2.;
        let (count, cell_lattice) = aabb.lattice(pre_compute_spacing);
        let acceleration_report = report.new_sub(ReportInfo {
            name: "Build Acceleration Structure".to_string(),
            completed_steps: 0,
            steps_to_completion: NonZero::new((count / REPORT_STRIDE).max(1)).unwrap(),
        });
        let cells: Vec<Cell> = cell_lattice
            .enumerate()
            .par_bridge()
            .map(|(i, center)| {
                ensure_err!(run.load(Ordering::Relaxed), MeshError::Cancelled);
                if i % REPORT_STRIDE == 0 {
                    acceleration_report.step();
                }
                let cell = self.triangles.iter().enumerate().fold(
                    Cell {
                        winding: 0.,
                        center,
                        close_triangles: Default::default(),
                    },
                    |mut cell, (triangle_idx, triangle)| {
                        let Some(n) = self.triangle_normals[triangle_idx].as_ref() else {
                            return cell;
                        };
                        let [a, b, c] = to_positions(triangle);
                        if close_measure < point_to_triangle(&cell.center, &a, &b, &c, n) {
                            cell.winding += wind_triangle(cell.center, triangle);
                        } else {
                            cell.close_triangles.push(triangle_idx);
                        }

                        cell
                    },
                );
                Ok(cell)
            })
            .collect::<Result<_, _>>()?;
        let average_ratio = cells
            .par_iter()
            .map(|cell| cell.close_triangles.len() as T / self.triangles.len() as T)
            .sum::<T>()
            / cells.len() as T;
        info!(average_ratio, "discretization optimization");
        if average_ratio > 0.1 {
            warn!("discretization optimization doesn't seem to work well for this setup");
        }
        drop(acceleration_report);

        let (count, samples_lattice) = aabb.lattice(spacing);
        let samples_report = report.new_sub(ReportInfo {
            name: "Create Samples".to_string(),
            completed_steps: 0,
            steps_to_completion: NonZero::new((count / REPORT_STRIDE).max(1)).unwrap(),
        });

        let samples = samples_lattice
            .enumerate()
            .par_bridge()
            .map(|(i, candidate)| {
                if i % REPORT_STRIDE == 0 {
                    samples_report.step();
                }
                candidate
            })
            .map(|on_lattice| {
                let random_offset =
                    (Vector3::new_random() - Vector3::repeat(0.5)) * spacing * randomness;
                on_lattice + random_offset
            })
            .filter(move |&candidate| {
                let closest_cell = cells
                    .iter()
                    .min_by(|a, b| {
                        let a = a.center - candidate;
                        let b = b.center - candidate;
                        a.norm_squared().total_cmp(&b.norm_squared())
                    })
                    .unwrap();
                let winding = closest_cell.winding
                    + closest_cell
                        .close_triangles
                        .iter()
                        .map(|triangle_idx| {
                            wind_triangle(candidate, &self.triangles[*triangle_idx])
                        })
                        .sum::<T>();
                winding > 0.5
            })
            .map(|candidate| {
                ensure_err!(run.load(Ordering::Relaxed), MeshError::Cancelled);
                Ok(candidate)
            })
            .collect::<Result<_, _>>()?;

        Ok(samples)
    }
}

fn point_to_line(p: &Vector3<T>, a: &Vector3<T>, b: &Vector3<T>) -> T {
    let p_a = p - a;
    let b_a = b - a;

    let ba = b_a.norm_squared();

    if ba < 1e-5 {
        return p_a.dot(&p_a).sqrt();
    }

    let d = p_a - b_a * (p_a.dot(&b_a) / ba).clamp(0., 1.);

    d.norm()
}

fn point_to_triangle(
    p: &Vector3<T>,
    a: &Vector3<T>,
    b: &Vector3<T>,
    c: &Vector3<T>,
    n: &Vector3<T>,
) -> T {
    let p_a = p - a;
    let p_b = p - b;
    let p_c = p - c;

    let b_a = b - a;
    let c_b = c - b;
    let a_c = a - c;

    if b_a.cross(n).dot(&p_a) < 0. && c_b.cross(n).dot(&p_b) < 0. && a_c.cross(n).dot(&p_c) < 0. {
        return p_a.dot(n);
    }

    [
        point_to_line(p, a, b),
        point_to_line(p, b, c),
        point_to_line(p, c, a),
    ]
    .into_iter()
    .min_by(T::total_cmp)
    .unwrap()
}
