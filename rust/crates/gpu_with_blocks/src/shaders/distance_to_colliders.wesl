// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{get_global_index, NORMALIZATION_EPS, perp, murmur_of_cell, Triangle};

@group(0) @binding(0)
var<storage, read_write> vertices: array<vec3f>;

@group(0) @binding(1)
var<storage, read_write> normals: array<vec3f>;

@group(0) @binding(2)
var<storage, read_write> velocities: array<vec3f>;

@group(0) @binding(3)
var<storage, read_write> triangles: array<Triangle>;

@group(0) @binding(4)
var<storage, read_write> opposite_vertices: array<Triangle>;

@group(0) @binding(5)
var<storage, read_write> frictions: array<f32>;

@group(0) @binding(6)
var<storage, read_write> block_ids: array<vec3i>;

@group(0) @binding(7)
var<storage, read_write> block_table: array<u32>;

@group(0) @binding(8)
var<storage, read_write> collider_bits: array<u32>;

@group(0) @binding(9)
var<storage, read_write> collider_offsets: array<u32>;

@group(0) @binding(10)
var<storage, read_write> collider_normals_and_distances: array<vec4f>;

@group(0) @binding(11)
var<storage, read_write> collider_velocities_and_frictions: array<vec4f>;

override WORKGROUP_SIZE: u32;
override CELL_SIZE: f32;
override LAYERS: u32;

var<immediate> collider_index: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let triangle_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id) / subgroup_size;
    if triangle_index >= arrayLength(&triangles) {
        return;
    }

    let table_mask = arrayLength(&block_table) - 1;

    let grid_node_size = CELL_SIZE / 2.;
    let side_margin_squared = pow(f32(LAYERS + 1) * grid_node_size, 2);
    let height_margin = f32(LAYERS) * grid_node_size;

    let triangle = triangles[triangle_index];
    let a = vertices[triangle.a];
    let b = vertices[triangle.b];
    let c = vertices[triangle.c];

    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

    let normal_area_2 = cross(-ab, ca);
    let area_2 = dot(normal_area_2, normal_area_2);

    if area_2 < NORMALIZATION_EPS * NORMALIZATION_EPS {
        return;
    }

    let n = normal_area_2 / sqrt(area_2);
    let offset = dot(n, a);

    var facing_axis = 0;
    if abs(n.y) > abs(n.x) {
        if abs(n.y) > abs(n.z) {
            facing_axis = 1;
        } else {
            facing_axis = 2;
        }
    } else if abs(n.z) > abs(n.x) {
        facing_axis = 2;
    }

    var plane_a = vec2(0.);
    var plane_b = vec2(0.);
    var plane_c = vec2(0.);
    var plane_n = vec2(0.);
    var normal_facing_coord = 0.;
    if facing_axis == 0 {
        plane_a = a.yz;
        plane_b = b.yz;
        plane_c = c.yz;
        plane_n = n.yz;
        normal_facing_coord = n.x;
    } else if facing_axis == 1 {
        plane_a = a.xz;
        plane_b = b.xz;
        plane_c = c.xz;
        plane_n = n.xz;
        normal_facing_coord = n.y;
    } else {
        plane_a = a.xy;
        plane_b = b.xy;
        plane_c = c.xy;
        plane_n = n.xy;
        normal_facing_coord = n.z;
    }

    let plane_ab = plane_a - plane_b;
    let plane_bc = plane_b - plane_c;
    let plane_ca = plane_c - plane_a;

    let plane_ab_ns = dot(plane_ab, plane_ab);
    let plane_bc_ns = dot(plane_bc, plane_bc);
    let plane_ca_ns = dot(plane_ca, plane_ca);

    if any(vec3(
        plane_ab_ns,
        plane_bc_ns,
        plane_ca_ns
    ) == vec3(0.)) {
        return;
    }

    let plane_min = vec2i(floor(min(plane_a, min(plane_b, plane_c)) / grid_node_size)) - vec2i(i32(LAYERS));
    let plane_max = vec2i(ceil(max(plane_a, max(plane_b, plane_c)) / grid_node_size)) + vec2i(i32(LAYERS));

    let plane_range = vec2u(plane_max - plane_min) + vec2u(1);
    let num_plane_samples = plane_range.x * plane_range.y;
    let plane_sample_rounds = num_plane_samples / subgroup_size;
    for (var plane_sample_round = 0u; plane_sample_round <= plane_sample_rounds; plane_sample_round++) {
        let plane_sample_index = plane_sample_round * subgroup_size + subgroup_invocation_id;
        if plane_sample_index >= num_plane_samples {
            break;
        }
        let plane_grid_node = plane_min + vec2i(vec2u(
            plane_sample_index / plane_range.y,
            plane_sample_index % plane_range.y,
        ));
        let plane_p = vec2f(plane_grid_node) * grid_node_size;

        let signs = vec3<bool>(
            perp(plane_p - plane_b, plane_ab) < 0.,
            perp(plane_p - plane_c, plane_bc) < 0.,
            perp(plane_p - plane_a, plane_ca) < 0.,
        );

        if any(signs) && any(!signs) && all(
            vec3f(
                squared_distance_to_segment(plane_p, plane_b, plane_ab, plane_ab_ns),
                squared_distance_to_segment(plane_p, plane_c, plane_bc, plane_bc_ns),
                squared_distance_to_segment(plane_p, plane_a, plane_ca, plane_ca_ns),
            ) >= vec3f(side_margin_squared)
        ) {
            continue;
        }

        var hits = (vec2f(offset - dot(plane_n, plane_p)) + vec2f(-height_margin, height_margin)) / normal_facing_coord;
        if hits.y < hits.x {
            hits = hits.yx;
        }
        let normal_facing_min = i32(floor(hits.x / grid_node_size));
        let normal_facing_max = i32(ceil(hits.y / grid_node_size));

        for (var normal_facing = normal_facing_min; normal_facing <= normal_facing_max; normal_facing++) {
            var grid_node_id = vec3i(0);
            if facing_axis == 0 {
                grid_node_id = vec3i(normal_facing, plane_grid_node.x, plane_grid_node.y);
            } else if facing_axis == 1 {
                grid_node_id = vec3i(plane_grid_node.x, normal_facing, plane_grid_node.y);
            } else {
                grid_node_id = vec3i(plane_grid_node.x, plane_grid_node.y, normal_facing);
            }

            let block_id = (grid_node_id + vec3i(1)) / 2;
            let block_index = get_block_index(block_id);
            if block_index == 0 {
                continue;
            }
            atomicOr(&collider_bits[block_index - 1], 1u << collider_index);
        }
    }
}

fn squared_distance_to_segment(point: vec2f, start: vec2f, segment: vec2f, squared_norm: f32) -> f32 {
    let factor = clamp(dot(point - start, segment) / squared_norm, 0., 1.);
    let projected = start + factor * segment;
    let to_point = projected - point;
    return dot(to_point, to_point);
}

fn get_block_index(block_id: vec3i) -> u32 {
    let hash = murmur_of_cell(block_id);
    let table_mask = arrayLength(&block_table) - 1;
    var slot = hash & table_mask;
    var block_index = 0u;
    loop {
        block_index = block_table[slot];
        if block_index == 0 || all(block_ids[block_index - 1] == block_id) {
            break;
        }
        slot += 1;
        slot &= table_mask;
    }
    return block_index;
}
