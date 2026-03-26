// This implementation of radix sort is heavily inspired by
// Harada, Takahiro, and Lee Howes. "Introduction to GPU radix sort." Heterogeneous Computing with OpenCL. Morgan Kaufman (2011).

use super::*;

#[cfg(test)]
mod test;

pub struct PrefixSort {
    bit_count: u32,
    count_subkeys: CountSubkeys,
    prefix_sum: PrefixSum,
    reorder: Reorder,
}

pub struct PrefixSortSettings {
    pub prefix_sum_workgroup_size: u32,
    pub count_subkeys_workgroup_size: u32,
    pub reorder_workgroup_size: u32,
    pub bit_count: u32,
}

pub struct PrefixSortBufferBindings<'a> {
    pub keys: wgpu::BufferBinding<'a>,
    pub indices: DoubleBuffer<'a>,

    pub counts: wgpu::BufferBinding<'a>,
    pub prefixes: wgpu::BufferBinding<'a>,
}

impl PrefixSort {
    pub fn new(
        context: &GpuContext,
        PrefixSortSettings {
            prefix_sum_workgroup_size,
            count_subkeys_workgroup_size,
            reorder_workgroup_size,
            bit_count,
        }: PrefixSortSettings,
    ) -> Self {
        let count_subkeys = CountSubkeys::new(context, count_subkeys_workgroup_size, bit_count);
        let prefix_sum = PrefixSum::new(context, prefix_sum_workgroup_size);
        let reorder = Reorder::new(context, reorder_workgroup_size, bit_count);

        Self {
            bit_count,
            count_subkeys,
            prefix_sum,
            reorder,
        }
    }

    pub fn min_counts(&self, key_count: u32) -> u32 {
        self.count_subkeys.min_counts(key_count)
    }
    pub fn min_prefixes(&self, key_count: u32) -> u32 {
        self.reorder.min_prefixes(key_count)
    }

    pub fn compute_in_pass(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        PrefixSortBufferBindings {
            keys,
            mut indices,
            counts,
            prefixes,
        }: PrefixSortBufferBindings,
    ) -> bool {
        for round in 0..32u32.div_ceil(self.bit_count) {
            let bit_offset = round * self.bit_count;

            self.count_subkeys.compute_in_pass(
                context,
                compute_pass,
                indices.front(),
                keys.clone(),
                counts.clone(),
                bit_offset,
            );
            self.prefix_sum.compute_in_pass(
                context,
                compute_pass,
                counts.clone(),
                prefixes.clone(),
            );
            self.reorder.compute_in_pass(
                context,
                compute_pass,
                ReorderBufferBindings {
                    keys: keys.clone(),
                    prefixes: prefixes.clone(),
                    indices_in: indices.front(),
                    indices_out: indices.back(),
                },
                bit_offset,
            );

            indices.swap();
        }

        !indices.swapped()
    }
}
