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

import super::util::{get_global_index, F32_MAX, NORMALIZATION_EPS, Triangle};

@group(0) @binding(0)
var<storage, read_write> test_positions: array<vec3f>;

@group(0) @binding(1)
var<storage, read_write> vertices: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> triangles: array<Triangle>;

@group(0) @binding(3)
var<storage, read_write> sdfs: array<f32>;

@group(0) @binding(4)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    if is_error() {
        return;
    }
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index == 0 {
        set_shader_id();
    }
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
    let n = normal_area_2 / area_2;

    let sa = dot(n, cross(bc, c - p)) > 0.;
    let sb = dot(n, cross(ca, a - p)) > 0.;
    let sc = dot(n, cross(ab, b - p)) > 0.;

    if sa && sb && sc {
        return abs(dot(p - a, n));
    }

    var distance: f32 = F32_MAX;
    if !sa {
        distance = min(distance, distance_to_segment(p, b, c));
    }
    if !sb {
        distance = min(distance, distance_to_segment(p, c, a));
    }
    if !sc {
        distance = min(distance, distance_to_segment(p, a, b));
    }
    return distance;
}

fn is_error() -> bool {
    return (atomicLoad(&status) & 0x0000FFFFu) != 0;
}

fn set_shader_id() {
    atomicAnd(&status, 0x0000FFFFu);
    atomicOr(&status, SHADER_ID << 16);
}
