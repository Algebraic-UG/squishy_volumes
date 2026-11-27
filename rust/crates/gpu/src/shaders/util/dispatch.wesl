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

fn get_global_index(workgroup_size: u32, num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * workgroup_size * num_workgroups.x) +
        (global_invocation_id.z * workgroup_size * num_workgroups.x * num_workgroups.y);
}

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

fn int_log(value: u32, base: u32) -> u32 {
    if base < 2u || value <= 1u {
        return 0u;
    }

    var result = 0u;
    var power = 1u;

    while power < value {
        result += 1u;

        // The next power would be >= value, without actually multiplying.
        if power > (value - 1u) / base {
            break;
        }

        power *= base;
    }

    return result;
}

fn int_pow(base: u32, exponent: u32) -> u32 {
    var result = 1u;
    for (var i = 0u; i < exponent; i++) {
        result *= base;
    }
    return result;
}

fn find_dispatch_xyz(dispatch_limit: u32, workgroup_size: u32, len: u32) -> vec3u {
    let workgroup_count = div_ceil(len, workgroup_size);
    let x = min(dispatch_limit, workgroup_count);
    let y = min(dispatch_limit, div_ceil(workgroup_count, dispatch_limit));
    let z = min(dispatch_limit, div_ceil(workgroup_count, dispatch_limit * dispatch_limit));
    return vec3u(x, y, z);
}

struct Indirect {
    x: u32,
    y: u32,
    z: u32,
    len: u32,
}
