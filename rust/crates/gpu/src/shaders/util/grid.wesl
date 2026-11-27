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

struct PositionAndColliderBits {
    position: vec3f,
    collider_bits: u32,
}

struct NodeIdAndColliderBits {
    node_id: vec3i,
    collider_bits: u32,
}

fn block_offset(block: u32) -> vec3i {
    var offset = vec3i(0);
    if (block & 1) == 1 {
        offset.x = 1;
    }
    if (block & 2) == 2 {
        offset.y = 1;
    }
    if (block & 4) == 4 {
        offset.z = 1;
    }
    return offset;
}

fn node_offset(node: u32) -> vec3i {
    switch node {
        case 0: { return vec3i(0, 0, 0); }
        case 1: { return vec3i(0, 0, 1); }
        case 2: { return vec3i(0, 0, 2); }
        case 3: { return vec3i(0, 1, 0); }
        case 4: { return vec3i(0, 1, 1); }
        case 5: { return vec3i(0, 1, 2); }
        case 6: { return vec3i(0, 2, 0); }
        case 7: { return vec3i(0, 2, 1); }
        case 8: { return vec3i(0, 2, 2); }
        case 9: { return vec3i(1, 0, 0); }
        case 10: { return vec3i(1, 0, 1); }
        case 11: { return vec3i(1, 0, 2); }
        case 12: { return vec3i(1, 1, 0); }
        case 13: { return vec3i(1, 1, 1); }
        case 14: { return vec3i(1, 1, 2); }
        case 15: { return vec3i(1, 2, 0); }
        case 16: { return vec3i(1, 2, 1); }
        case 17: { return vec3i(1, 2, 2); }
        case 18: { return vec3i(2, 0, 0); }
        case 19: { return vec3i(2, 0, 1); }
        case 20: { return vec3i(2, 0, 2); }
        case 21: { return vec3i(2, 1, 0); }
        case 22: { return vec3i(2, 1, 1); }
        case 23: { return vec3i(2, 1, 2); }
        case 24: { return vec3i(2, 2, 0); }
        case 25: { return vec3i(2, 2, 1); }
        default: { return vec3i(2, 2, 2); }
    }
}

fn position_to_low_node(node_size: f32, position: vec3f) -> vec3i {
    return vec3i(round(position / node_size)) - vec3i(1);
}

fn kernel_linear(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. { return 1. - x; } else { return 0.; }
}

fn kernel_quadratic(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. / 2. {
        return 3. / 4. - x * x;
    } else if x < 3. / 2. {
        return 1. / 2. * (3. / 2. - x) * (3. / 2. - x);
    } else {
        return 0.;
    }
}

fn kernel_cubic(signed_x: f32) -> f32 {
    let x = abs(signed_x);
    if x < 1. {
        return 1. / 2. * x * x * x - x * x + 2. / 3.;
    } else if x < 2. {
        return 1. / 6. * (2. - x) * (2. - x) * (2. - x);
    } else {
        return 0.;
    }
}
