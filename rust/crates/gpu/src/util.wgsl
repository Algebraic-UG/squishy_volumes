override WORKGROUP_SIZE: u32;
override DISPATCH_LIMIT: u32;

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
