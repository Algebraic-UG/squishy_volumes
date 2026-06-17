use std::num::NonZeroU32;

use super::*;

use nalgebra::Vector4;

#[cfg(test)]
mod test;

pub struct FindCellBoundaries {
    find_cell_boundaries: CompiledModule,
}

pub struct Settings {
    pub workgroup_size: NonZeroU32,
    pub cell_size: f32,
}

pub struct Parameters;

pub struct Input {
    pub indirect: Allocation,
    pub positions: Allocation,
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        workgroup_size: NonZeroU32,
        dispatch_limit: NonZeroU32,
        positions: &[Vector4<f32>],
    ) -> Self {
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size,
            dispatch_limit,
            len: positions.len() as u32,
        });

        let positions = Allocation::new(device, "positions", positions);
        let indirect = Allocation::new(device, "indirect", &[indirect]);

        Self {
            indirect,
            positions,
        }
    }
}

pub struct Output {
    pub boundaries: Allocation,
}

impl PipelinePart for FindCellBoundaries {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(
        context: &GpuContext,
        Settings {
            workgroup_size,
            cell_size,
        }: Settings,
    ) -> Self {
        let workgroup_size = workgroup_size.get();
        assert!(cell_size > 0.);

        let device = context.device();
        let_compiled_module!(
            find_cell_boundaries,
            CompiledModuleSettings {
                device,
                bind_group_entries: [
                    (Indirect::MIN_BINDING_SIZE, true),
                    (Vector4::<f32>::MIN_BINDING_SIZE, false),
                    (u32::MIN_BINDING_SIZE, false),
                ],
                immediate_size: 0,
                constants: [
                    ("WORKGROUP_SIZE", workgroup_size as f64),
                    ("CELL_SIZE", cell_size as f64),
                ],
            }
        );

        Self {
            find_cell_boundaries,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            indirect,
            positions,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let boundaries = context
            .allocator()?
            .allocate::<u32>("boundaries", positions.len::<Vector4<f32>>())?;

        let mut compute_pass = encoder.begin_compute_pass(self.find_cell_boundaries.label);
        compute_pass.set_pipeline(&self.find_cell_boundaries.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.find_cell_boundaries,
                [
                    indirect.binding(),
                    positions.binding(),
                    boundaries.binding(),
                ],
            ),
            &[],
        );
        compute_pass.dispatch_workgroups_indirect(indirect.buffer(), indirect.offset());

        Ok(Output { boundaries })
    }
}
