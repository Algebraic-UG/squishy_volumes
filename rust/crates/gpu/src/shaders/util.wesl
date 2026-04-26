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

fn position_to_cell(cell_size: f32, position: vec3f) -> vec3i {
    return vec3i(floor(position / cell_size + vec3f(0.25)));
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

struct ParticleParameters {
    flags: u32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
}

fn invariant_2(position_gradient: mat3x3f) -> f32 {
    return dot(position_gradient[0], position_gradient[0]) +
        dot(position_gradient[1], position_gradient[1]) +
        dot(position_gradient[2], position_gradient[2]);
}

fn invariant_3(position_gradient: mat3x3f) -> f32 {
    return determinant(position_gradient);
}

fn elastic_energy_neo_hookean_by_invariants(
    mu: f32,
    lambda: f32,
    invariant_2: f32,
    invariant_3: f32,
) -> f32 {
    return mu / 2. * (invariant_2 - 3.) - mu * log(invariant_3) + lambda / 2. * log(invariant_3) * log(invariant_3);
}

fn elastic_energy_neo_hookean(
    mu: f32,
    lambda: f32,
    position_gradient: mat3x3f,
) -> f32 {
    return elastic_energy_neo_hookean_by_invariants(
        mu, lambda, invariant_2(position_gradient), invariant_3(position_gradient)
    );
}

fn elastic_energy_inviscid_by_invariant(bulk_modulus: f32, exponent: f32, invariant_3: f32) -> f32 {
    let at_rest = bulk_modulus * (1. - 1. / (1. - exponent));
    return bulk_modulus * (invariant_3 - pow(invariant_3, 1. - exponent) / (1. - exponent)) - at_rest;
}

fn elastic_energy_inviscid(
    bulk_modulus: f32,
    exponent: f32,
    position_gradient: mat3x3f,
) -> f32 {
    return elastic_energy_inviscid_by_invariant(bulk_modulus, exponent, invariant_3(position_gradient));
}

fn partial_elastic_energy_neo_hookean_by_invariant_2(mu: f32) -> f32 {
    return mu / 2.;
}

fn partial_invariant_2_by_position_gradient(position_gradient: mat3x3f) -> mat3x3f {
    return 2. * position_gradient;
}

fn partial_elastic_energy_neo_hookean_by_invariant_3(mu: f32, lambda: f32, invariant_3: f32) -> f32 {
    return (lambda * log(invariant_3) - mu) / invariant_3;
}

fn partial_invariant_3_by_position_gradient(position_gradient: mat3x3f) -> mat3x3f {
    return mat3x3f(
        cross(position_gradient[1], position_gradient[2]),
        cross(position_gradient[2], position_gradient[0]),
        cross(position_gradient[0], position_gradient[1]),
    );
}

fn first_piola_stress_neo_hookean(
    mu: f32,
    lambda: f32,
    position_gradient: mat3x3f,
) -> mat3x3f {
    return partial_elastic_energy_neo_hookean_by_invariant_2(mu)
    * partial_invariant_2_by_position_gradient(position_gradient)
    + partial_elastic_energy_neo_hookean_by_invariant_3(
        mu,
        lambda,
        invariant_3(position_gradient),
    ) * partial_invariant_3_by_position_gradient(position_gradient);
}

fn partial_elastic_energy_inviscid_by_invariant_3(
    bulk_modulus: f32,
    exponent: f32,
    invariant_3: f32,
) -> f32 {
    return bulk_modulus * (1. - 1. / pow(invariant_3, exponent));
}

fn first_piola_stress_inviscid(
    bulk_modulus: f32,
    exponent: f32,
    position_gradient: mat3x3f,
) -> mat3x3f {
    return partial_elastic_energy_inviscid_by_invariant_3(
        bulk_modulus,
        exponent,
        invariant_3(position_gradient),
    ) * partial_invariant_3_by_position_gradient(position_gradient);
}
