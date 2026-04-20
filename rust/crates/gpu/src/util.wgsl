override WORKGROUP_SIZE: u32;
override DISPATCH_LIMIT: u32;
override CELL_SIZE: f32;

fn get_global_index(num_workgroups: vec3<u32>, global_invocation_id: vec3<u32>) -> u32 {
    return global_invocation_id.x +
        (global_invocation_id.y * WORKGROUP_SIZE * num_workgroups.x) +
        (global_invocation_id.z * WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y);
}

fn div_ceil(left: u32, right: u32) -> u32 {
    if left == 0 {
        return 0;
    }
    return 1 + ((left - 1) / right);
}

fn int_log(value: u32, base: u32) -> u32 {
    if base < 2 {
        return 0;
    }
    var result = 1u;
    var power = 1u;
    while power < value {
        result += 1;
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

fn find_dispatch_xyz(len: u32) -> vec3u {
    let workgroup_count = div_ceil(len, WORKGROUP_SIZE);
    let x = min(DISPATCH_LIMIT, workgroup_count);
    let y = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT));
    let z = min(DISPATCH_LIMIT, div_ceil(workgroup_count, DISPATCH_LIMIT * DISPATCH_LIMIT));
    return vec3u(x, y, z);
}

struct Indirect {
    x: u32,
    y: u32,
    z: u32,
    len: u32,
}

// This is derived from https://github.com/stusmall/murmur3
// and tested against it

fn rotl32(x: u32, r: u32) -> u32 {
    return (x << r) | (x >> (32u - r));
}

fn fmix32(h_in: u32) -> u32 {
    var h = h_in;
    h = h ^ (h >> 16u);
    h = h * 0x85ebca6bu;
    h = h ^ (h >> 13u);
    h = h * 0xc2b2ae35u;
    h = h ^ (h >> 16u);
    return h;
}

fn murmur3_mix_block(h1_in: u32, k1_in: u32) -> u32 {
    let c1: u32 = 0xcc9e2d51u;
    let c2: u32 = 0x1b873593u;

    var h1 = h1_in;
    var k1 = k1_in;

    k1 = k1 * c1;
    k1 = rotl32(k1, 15u);
    k1 = k1 * c2;

    h1 = h1 ^ k1;
    h1 = rotl32(h1, 13u);
    h1 = h1 * 5u + 0xe6546b64u;

    return h1;
}

fn murmur3_x86_32_3u32(a: u32, b: u32, c: u32, seed: u32) -> u32 {
    var h1 = seed;

    h1 = murmur3_mix_block(h1, a);
    h1 = murmur3_mix_block(h1, b);
    h1 = murmur3_mix_block(h1, c);

    h1 = h1 ^ 12u;

    return fmix32(h1);
}

fn murmur_of_cell(cell: vec3i) -> u32 {
    return murmur3_x86_32_3u32(
        i32_to_ordered_u32(cell.x),
        i32_to_ordered_u32(cell.y),
        i32_to_ordered_u32(cell.z),
        0,
    );
}

fn i32_to_ordered_u32(x: i32) -> u32 {
    return bitcast<u32>(x) ^ 0x80000000u;
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

fn position_to_cell(position: vec3f) -> vec3i {
    return vec3i(floor(position / CELL_SIZE + vec3f(CELL_SIZE * 0.25)));
}
