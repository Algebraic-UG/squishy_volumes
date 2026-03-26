//https://github.com/gfx-rs/wgpu/issues/8202
//enable subgroups;

@group(0) @binding(0)
var<storage, read> indices: array<u32>;

@group(0) @binding(1)
var<storage, read> keys: array<u32>;

@group(0) @binding(2)
var<storage, read_write> counts: array<u32>;

var<immediate> bit_offset: u32;

override WORKGROUP_SIZE: u32;
override BIT_COUNT: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(subgroup_size) subgroup_size: u32,
    @builtin(num_subgroups) num_subgroups: u32,
    @builtin(subgroup_invocation_id) subgroup_invocation_id: u32,
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    // The global_index isn't supported yet.
    let global_stride = vec3(
        1,
        WORKGROUP_SIZE * num_workgroups.x,
        WORKGROUP_SIZE * num_workgroups.x * num_workgroups.y,
    );
    let global_index = dot(global_invocation_id, global_stride);

    let array_length = arrayLength(&keys);

    let counter_count = 1u << BIT_COUNT;
    let mask = counter_count - 1;

    let key_valid = global_index < array_length;

    var key = 0u;
    if key_valid {
        key = keys[indices[global_index]];
    }

    let sub_key = (key >> bit_offset) & mask;

    var my_count = 0u;
    for (var counter: u32 = 0u; counter < counter_count; counter++) {
        var sub_key_count = 0u;
        if key_valid && sub_key == counter {
            sub_key_count = 1u;
        }

        sub_key_count = subgroupAdd(sub_key_count);

        if subgroup_invocation_id == counter {
            my_count = sub_key_count;
        }
    }

    if subgroup_invocation_id < counter_count {
        let subgroup_index = global_index / subgroup_size;
        let total_num_subgroups = num_subgroups * num_workgroups.x * num_workgroups.y * num_workgroups.z;
        let count_index = total_num_subgroups * subgroup_invocation_id + subgroup_index;
        counts[count_index] = my_count;
    }
}
