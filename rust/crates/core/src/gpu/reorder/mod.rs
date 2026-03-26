#[cfg(test)]
mod test;

use std::num::NonZeroU64;

use super::*;

pub struct Reorder {
    workgroup_size: u32,
    subgroup_size: u32,
    bit_count: u32,
    compiled_module: CompiledModule,
}

pub struct ReorderBufferBindings<'a> {
    pub keys: wgpu::BufferBinding<'a>,
    pub prefixes: wgpu::BufferBinding<'a>,
    pub indices_in: wgpu::BufferBinding<'a>,
    pub indices_out: wgpu::BufferBinding<'a>,
}

impl Reorder {
    pub fn new(context: &GpuContext, workgroup_size: u32, bit_count: u32) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));
        assert!(bit_count > 0);
        assert!(subgroup_size >= 2u32.pow(bit_count));

        let device = context.device();

        let label = Some("reorder");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_u32_entry(0, true),
                bind_group_layout_u32_entry(1, true),
                bind_group_layout_u32_entry(2, true),
                bind_group_layout_u32_entry(3, false),
            ],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size: 4,
                }),
            ),
            module: &device.create_shader_module(wgpu::include_wgsl!("reorder.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("BIT_COUNT", bit_count as f64),
                ],
                ..Default::default()
            },
            cache: None,
        });

        let compiled_module = CompiledModule {
            label,
            bind_group_layout,
            compute_pipeline,
        };

        Self {
            workgroup_size,
            subgroup_size,
            bit_count,
            compiled_module,
        }
    }

    pub fn min_prefixes(&self, key_count: u32) -> u32 {
        let subgroups_per_workgroup = self.workgroup_size / self.subgroup_size;
        let workgroup_count = key_count.div_ceil(self.workgroup_size);
        let actual_workgroup_count = find_x_y_z(workgroup_count).into_iter().product::<u32>();
        actual_workgroup_count * subgroups_per_workgroup * 2u32.pow(self.bit_count)
    }

    pub fn compute_in_pass(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        ReorderBufferBindings {
            keys,
            prefixes,
            indices_in,
            indices_out,
        }: ReorderBufferBindings,
        bit_offset: u32,
    ) {
        let device = context.device();

        let element_size = NonZeroU64::new(4).unwrap();
        let key_count = elements_in_binding(&element_size, &keys);
        assert!(key_count == elements_in_binding(&element_size, &indices_in));
        assert!(key_count == elements_in_binding(&element_size, &indices_out));
        let prefix_count = elements_in_binding(&element_size, &prefixes);
        assert!(prefix_count.get() >= self.min_prefixes(key_count.get()));

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(keys),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(prefixes),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(indices_in),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Buffer(indices_out),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        compute_pass.set_immediates(0, bytemuck::bytes_of(&bit_offset));

        let workgroup_count = key_count.get().div_ceil(self.workgroup_size);
        let [x, y, z] = find_x_y_z(workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
