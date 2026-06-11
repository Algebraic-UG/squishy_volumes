// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, F32_MAX, NORMALIZATION_EPS, Triangle};

@group(0) @binding(0)
var<storage, read_write> test_positions: array<vec3f>;

@group(0) @binding(1)
var<storage, read_write> vertices: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> triangles: array<Triangle>;

@group(0) @binding(3)
var<storage, read_write> sdfs: array<f32>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&test_positions) {
        return;
    }

    let p = test_positions[global_index];

    var sdf: f32 = F32_MAX;
    for (var triangle_index = 0u; triangle_index < arrayLength(&triangles); triangle_index++) {
        let triangle = triangles[triangle_index];
        let a = vertices[triangle.a];
        let b = vertices[triangle.b];
        let c = vertices[triangle.c];

        sdf = min(sdf, distance_to_triangle(p, a, b, c));
    }
    sdfs[global_index] = sdf;
}

fn distance_to_segment(p: vec3f, start: vec3f, end: vec3f) -> f32 {
    let segment = end - start;
    let along_segment = dot(p - start, segment) / dot(segment, segment);
    if along_segment < 0. {
        return length(p - start);
    } else if along_segment < 1. {
        return length(p - start - segment * along_segment);
    } else {
        return length(p - end);
    }
}

fn distance_to_triangle(p: vec3f, a: vec3f, b: vec3f, c: vec3f) -> f32 {
    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

    let normal_area_2 = cross(-ab, ca);
    let area_2 = length(normal_area_2);
    if area_2 < NORMALIZATION_EPS {
        return F32_MAX;
    }
    let normal = normal_area_2 / area_2;

    let sa = dot(b - p, cross(normal, ab)) < 0.;
    let sb = dot(c - p, cross(normal, bc)) < 0.;
    let sc = dot(a - p, cross(normal, ca)) < 0.;

    if sa && sb && sc {
        return abs(dot(p - a, normal));
    } else if sa {
        return distance_to_segment(p, b, c);
    } else if sb {
        return distance_to_segment(p, c, a);
    } else {
        return distance_to_segment(p, a, b);
    }
}
