use nalgebra::Vector4;
use wgpu::util::DeviceExt as _;

use super::*;

#[cfg(test)]
mod test;

pub struct FindCellBoundaries {
    workgroup_size: u32,
    compiled_module: CompiledModule,
}

pub struct FindCellBoundariesSettings {
    pub workgroup_size: u32,
    pub cell_size: f32,
}

pub struct FindCellBoundariesBufferInput<'a> {
    pub positions: &'a [Vector4<f32>],
}

pub struct FindCellBoundariesBuffers {
    pub positions: wgpu::Buffer,
    pub boundaries: wgpu::Buffer,
}

pub struct FindCellBoundariesBufferBindings<'a> {
    pub positions: wgpu::BufferBinding<'a>,
    pub boundaries: wgpu::BufferBinding<'a>,
}

impl<'a> From<&'a FindCellBoundariesBuffers> for FindCellBoundariesBufferBindings<'a> {
    fn from(
        FindCellBoundariesBuffers {
            positions,
            boundaries,
        }: &'a FindCellBoundariesBuffers,
    ) -> Self {
        Self {
            positions: positions.as_entire_buffer_binding(),
            boundaries: boundaries.as_entire_buffer_binding(),
        }
    }
}

impl PipelinePart for FindCellBoundaries {
    type Settings = FindCellBoundariesSettings;
    type Parameters = ();
    type BufferInput<'a> = FindCellBoundariesBufferInput<'a>;
    type Buffers = FindCellBoundariesBuffers;
    type BufferBindings<'a> = FindCellBoundariesBufferBindings<'a>;

    fn new(
        context: &GpuContext,
        Self::Settings {
            workgroup_size,
            cell_size,
        }: Self::Settings,
    ) -> Self {
        let subgroup_size = context.subgroup_size().get();
        assert!(workgroup_size > 0);
        assert!(workgroup_size.is_multiple_of(subgroup_size));
        assert!(cell_size > 0.);

        let device = context.device();
        let label = Some("find_cell_boundaries");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &[
                bind_group_layout_entry::<Vector4<f32>>(0, true),
                bind_group_layout_entry::<u32>(1, false),
            ],
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    ..Default::default()
                }),
            ),
            module: &device.create_shader_module(wgpu::include_wgsl!("find_cell_boundaries.wgsl")),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &[
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("CELL_SIZE", cell_size as f64),
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
            compiled_module,
        }
    }

    fn create_buffers<'a>(
        &self,
        context: &GpuContext,
        Self::BufferInput { positions }: Self::BufferInput<'a>,
    ) -> Self::Buffers {
        let n = positions.len();

        let device = context.device();
        let positions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("positions"),
            contents: bytemuck::cast_slice(positions),
            usage: wgpu::BufferUsages::STORAGE,
        });

        let boundaries = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("counts"),
            size: n as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        Self::Buffers {
            positions,
            boundaries,
        }
    }

    fn compute_in_pass<'a>(
        &self,
        context: &GpuContext,
        compute_pass: &mut wgpu::ComputePass,
        Self::BufferBindings {
            positions,
            boundaries,
        }: Self::BufferBindings<'a>,
        _: Self::Parameters,
    ) {
        let position_count = elements_in_binding::<Vector4<f32>>(&positions);
        let boundary_count = elements_in_binding::<u32>(&boundaries);
        assert_eq!(position_count, boundary_count);

        let device = context.device();

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.compiled_module.label,
            layout: &self.compiled_module.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(positions.clone()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(boundaries.clone()),
                },
            ],
        });

        compute_pass.set_pipeline(&self.compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);
        let workgroup_count = position_count.get().div_ceil(self.workgroup_size);
        let [x, y, z] = find_x_y_z(workgroup_count);
        compute_pass.dispatch_workgroups(x, y, z);
    }
}
