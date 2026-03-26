use rand::{SeedableRng as _, rngs::ChaCha8Rng, seq::SliceRandom as _};

use crate::gpu::GpuContext;

pub const MAX_NUM_PARTICLES: u32 = 10000000;

pub fn prefix_sum_on_cpu(input: &[u32]) -> Vec<u32> {
    input
        .iter()
        .scan(0, |prefix_sum, item| {
            let result = Some(*prefix_sum);
            *prefix_sum += item;
            result
        })
        .collect()
}

// This one is ugly.
// We're emulating the behaviour on the GPU which is influenced by the fact that we have to
// dispatch in multiples of the workgroup size.
// Given that the workgroup size is a multiple of the subgroup size, there can be subgroups
// that are entirely out of bounds.
pub fn count_subkeys_on_cpu(
    bit_count: u32,
    bit_offset: u32,
    workgroup_size: u32,
    subgroup_size: u32,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    use crate::gpu::find_x_y_z;

    let counter_count = 1 << bit_count;
    let mask = counter_count - 1;

    // this part calculates how many counters there will be
    let subgroups_per_workgroup = workgroup_size / subgroup_size;
    let workgroup_count = (keys.len() as u32).div_ceil(workgroup_size);
    let actual_workgroup_count = find_x_y_z(workgroup_count).into_iter().product::<u32>();
    let num_subgroups = actual_workgroup_count * subgroups_per_workgroup;
    let num_counter = num_subgroups * counter_count;

    // we can chunk through the subgroups but it's not enough
    let mut counts: Vec<u32> = indices
        .chunks(subgroup_size as usize)
        .flat_map(|chunk| {
            (0..counter_count).map(|counter| {
                chunk
                    .iter()
                    .map(|index| keys[*index as usize])
                    .map(|number| (number >> bit_offset) & mask)
                    .filter(move |sub_key| *sub_key == counter)
                    .count() as u32
            })
        })
        .collect();

    // the missing entries correspond to subgroups that are entirely out of bounds
    // they count all zero
    counts.resize(num_counter as usize, 0);

    // this part transposes the data
    let counts = &counts;
    (0..counter_count)
        .flat_map(|counter| {
            (0..num_subgroups)
                .map(move |subgroup| counts[(subgroup * counter_count + counter) as usize])
        })
        .collect()
}

pub fn shuffle<T>(v: &mut [T], seed: u64) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    v.shuffle(&mut rng);
}

pub fn get_subgroup_size() -> u32 {
    GpuContext::new(MAX_NUM_PARTICLES)
        .unwrap()
        .subgroup_size()
        .get()
}

pub fn sort_on_cpu_by_bits(
    bit_count: u32,
    bit_offset: u32,
    indices: &[u32],
    keys: &[u32],
) -> Vec<u32> {
    let counter_count = 1 << bit_count;
    let mask = counter_count - 1;

    let mut indices = indices.to_vec();

    indices.sort_by_key(|index| {
        let key = keys[*index as usize];
        (key >> bit_offset) & mask
    });

    indices
}
