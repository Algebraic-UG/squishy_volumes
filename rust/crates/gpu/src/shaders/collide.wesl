// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{PositionAndColliderBits, get_global_index, F32_MAX, U32_MAX, NORMALIZATION_EPS, Triangle, try_normalize, sign};

struct Opposite {
    ab: u32,
    bc: u32,
    ca: u32,
}

struct BvhMeta {
    level: u32,
    leaf_size: f32,
    offset_x: i32,
    offset_y: i32,
    offset_z: i32,
}

// indexed by particle

@group(0) @binding(0)
var<storage, read_write> particle_positions_and_collider_bits: array<PositionAndColliderBits>;

@group(0) @binding(1)
var<storage, read_write> particle_velocities: array<vec3f>;

// indexed by vertex

@group(0) @binding(2)
var<storage, read_write> vertex_positions: array<vec3f>;

@group(0) @binding(3)
var<storage, read_write> vertex_normals: array<vec3f>;

// indexed by triangle 

@group(0) @binding(4)
var<storage, read_write> triangle_indices: array<Triangle>;

@group(0) @binding(5)
var<storage, read_write> triangle_collider: array<u32>;

@group(0) @binding(6)
var<storage, read_write> triangle_normals: array<vec3f>;

@group(0) @binding(7)
var<storage, read_write> triangle_opposites: array<Opposite>;

@group(0) @binding(8)
var<storage, read_write> triangle_frictions: array<f32>;

// acceleration sturcture

@group(0) @binding(9)
var<storage, read_write> bvh_meta: BvhMeta;

@group(0) @binding(10)
var<storage, read_write> bvh_nodes: array<u32>;

@group(0) @binding(11)
var<storage, read_write> bvh_indices: array<u32>;

@group(0) @binding(12)
var<storage, read_write> status: atomic<u32>;

override WORKGROUP_SIZE: u32;
override SHADER_ID: u32;
override FORGET_DISTANCE: f32;
override ACCEPT_DISTANCE: f32;
override TIME_STEP: f32;

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
    if global_index >= arrayLength(&particle_positions_and_collider_bits) {
        return;
    }

    let p = particle_positions_and_collider_bits[global_index].position;

    let triangle_indices_start_end = find_relevant_triangle_indices(p);

    if triangle_indices_start_end.x == triangle_indices_start_end.y {
        particle_positions_and_collider_bits[global_index].collider_bits = 0;
        return;
    }

    let closest_triangle_per_collider = find_closest_triangle_per_collider(p, triangle_indices_start_end);

    var velocity = particle_velocities[global_index];
    var collider_bits = particle_positions_and_collider_bits[global_index].collider_bits;
    var collider = 0u;
    loop {
        if !(collider < 16) {
            break;
        }
        var bits = 0u;

        let closest_triangle = closest_triangle_per_collider[collider];
        if closest_triangle == U32_MAX {
            continue;
        }

        let triangle = triangle_indices[closest_triangle];
        let opposites = triangle_opposites[closest_triangle];
        var normal = triangle_normals[closest_triangle];

        let a = vertex_positions[triangle.a];
        let b = vertex_positions[triangle.b];
        let c = vertex_positions[triangle.c];

        let ab = a - b;
        let bc = b - c;
        let ca = c - a;

        let area_2 = dot(normal, cross(ca, ab));
        let bary_a = dot(normal, cross(bc, c - p)) / area_2;
        let bary_b = dot(normal, cross(ca, a - p)) / area_2;
        let bary_c = dot(normal, cross(ab, b - p)) / area_2;

        let sa = bary_a > 0.;
        let sb = bary_b > 0.;
        let sc = bary_c > 0.;

        var to_p = vec3f(0.);
        if sa && sb && sc {
            to_p = normal * dot(p - a, normal);
        } else {
            let ab_f = dot(p - b, ab) / dot(ab, ab);
            let bc_f = dot(p - c, bc) / dot(bc, bc);
            let ca_f = dot(p - a, ca) / dot(ca, ca);
            if !sa && bc_f >= 0. && bc_f <= 1. {
                if opposites.bc == U32_MAX {
                    continue;
                }
                normal += triangle_normals[opposites.bc];
                to_p = p - c - bc * bc_f;
            } else if !sb && ca_f >= 0. && ca_f <= 1. {
                if opposites.ca == U32_MAX {
                    continue;
                }
                normal += triangle_normals[opposites.ca];
                to_p = p - a - ca * ca_f;
            } else if !sc && ab_f >= 0. && ab_f <= 1. {
                if opposites.ab == U32_MAX {
                    continue;
                }
                normal += triangle_normals[opposites.ab];
                to_p = p - b - ab * ab_f;
            } else if bary_a >= 1. {
                normal = vertex_normals[triangle.a];
                to_p = p - a;
            } else if bary_b >= 1. {
                normal = vertex_normals[triangle.b];
                to_p = p - b;
            } else { //if bary_c >= 1. {
                normal = vertex_normals[triangle.c];
                to_p = p - c;
            }
            if all(normal == vec3f(0.)) {
                continue;
            }
        }

        let distance = length(to_p);
        let new_side = 0 <= dot(normal, to_p);
        if (collider_bits & (0x00010000u << collider)) == 0 {
            if distance < ACCEPT_DISTANCE {
                if new_side {
                    bits = 0x00010001u << collider;
                } else {
                    bits = 0x00010000u << collider;
                }
            }
            continue;
        }

        bits = collider_bits;

        if ((collider_bits & (1u << collider)) != 0) == new_side {
            continue;
        }

        if distance > NORMALIZATION_EPS {
            let to_p_normalized = to_p / distance;
            let tangential = velocity - to_p_normalized * dot(velocity, to_p_normalized);
            velocity -= tangential * min(distance * triangle_frictions[closest_triangle], 1.);
        }

        velocity -= to_p / TIME_STEP;

        continuing {
            collider_bits &= ~(0x00010001u << collider);
            collider_bits |= bits;
            collider++;
        }
    }

    particle_positions_and_collider_bits[global_index].collider_bits = collider_bits;
    particle_velocities[global_index] = velocity;
}

struct EdgeResult {
    valid: bool,
    distance: f32,
    to_p: vec3f,
}

fn find_relevant_triangle_indices(p: vec3f) -> vec2u {
    let i = vec3i(floor(p / bvh_meta.leaf_size)) - vec3i(bvh_meta.offset_x, bvh_meta.offset_y, bvh_meta.offset_z);
    if any(i < vec3i(0)) || any(i >= vec3i(1 << (bvh_meta.level * 2))) {
        return vec2u(0, 0);
    }
    let q = vec3u(i);

    var node = 0u;
    for (var level = bvh_meta.level - 1u; level >= 0; level--) {
        let child_bits_00_32 = bvh_nodes[node];
        let child_bits_32_64 = bvh_nodes[node + 1];
        if child_bits_00_32 == 0 && child_bits_32_64 == 0 {
            break;
        }
        let child = //
              (((q.x >> (2 * level)) & 3) << 4)
            | (((q.y >> (2 * level)) & 3) << 2)
            | (((q.z >> (2 * level)) & 3) << 0);

        var offset = 0u;
        if child < 32 {
            if (child_bits_00_32 & (1u << child)) == 0 {
                return vec2u(0, 0);
            }
            offset += countOneBits(child_bits_00_32 & ((1u << child) - 1));
        } else {
            if (child_bits_32_64 & (1u << (child - 32))) == 0 {
                return vec2u(0, 0);
            }
            offset += countOneBits(child_bits_00_32);
            offset += countOneBits(child_bits_32_64 & ((1u << (child - 32)) - 1));
        }
        node = bvh_nodes[node + 2 + offset];
    }

    return vec2u(
        bvh_nodes[node + 2],
        bvh_nodes[node + 3],
    );
}

fn find_closest_triangle_per_collider(p: vec3f, triangle_indices_start_end: vec2u) -> array<u32, 16> {
    var closest_triangle_per_collider: array<u32, 16> = array<u32, 16>(
        U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX,
        U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX, U32_MAX,
    );
    var min_distance_per_collider: array<f32, 16> = array<f32,16>(
        F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX,
        F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX, F32_MAX,
    );
    for (var lookup = triangle_indices_start_end.x; lookup < triangle_indices_start_end.y; lookup++) {
        let triangle_index = bvh_indices[lookup];

        let n = triangle_normals[triangle_index];
        if all(n == vec3f(0.)) {
            continue;
        }

        let collider = triangle_collider[triangle_index];
        let triangle = triangle_indices[triangle_index];
        let a = vertex_positions[triangle.a];
        let b = vertex_positions[triangle.b];
        let c = vertex_positions[triangle.c];

        let distance = distance_to_triangle(p, a, b, c, n);
        if distance < FORGET_DISTANCE && distance < min_distance_per_collider[collider] {
            min_distance_per_collider[collider] = distance;
            closest_triangle_per_collider[collider] = triangle_index;
        }
    }

    return closest_triangle_per_collider;
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

fn distance_to_triangle(p: vec3f, a: vec3f, b: vec3f, c: vec3f, n: vec3f) -> f32 {
    let ab = a - b;
    let bc = b - c;
    let ca = c - a;

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
