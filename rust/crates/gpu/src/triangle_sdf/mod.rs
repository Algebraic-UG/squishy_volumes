// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

#[cfg(test)]
mod test;

use std::num::NonZeroU32;

use nalgebra::Vector4;

use super::*;

pub struct TriangleSdf {
    workgroup_size: NonZeroU32,
    triangle_sdf: CompiledModule,
}

#[derive(Clone, Copy)]
pub struct Settings {
    pub workgroup_size: NonZeroU32,
}

pub struct Parameters;

pub struct Input {
    pub test_positions: Allocation,
    pub vertices: Allocation,
    pub triangles: Allocation,
}

pub struct InputData<'a> {
    pub test_positions: &'a [Vector4<f32>],
    pub vertices: &'a [Vector4<f32>],
    pub triangles: &'a [Triangle],
}

impl Input {
    pub fn new(
        device: &wgpu::Device,
        InputData {
            test_positions,
            vertices,
            triangles,
        }: InputData,
    ) -> Self {
        assert!(triangles.iter().all(|triangle| {
            triangle.a < vertices.len() as u32
                && triangle.b < vertices.len() as u32
                && triangle.c < vertices.len() as u32
        }));

        let test_positions = Allocation::new(device, "test_positions", test_positions);
        let vertices = Allocation::new(device, "vertice", vertices);
        let triangles = Allocation::new(device, "triangles", triangles);

        Self {
            test_positions,
            vertices,
            triangles,
        }
    }
}
pub struct Output {
    pub sdf: Allocation,
}

impl PipelinePart for TriangleSdf {
    type Settings = Settings;
    type Parameters = Parameters;
    type Input = Input;
    type Output = Output;

    fn new(context: &GpuContext, Settings { workgroup_size }: Settings) -> Self {
        let_compiled_module!(
            triangle_sdf,
            CompiledModuleSettings {
                device: context.device(),
                bind_group_entries: [
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // test_positions
                    (Vector4::<f32>::MIN_BINDING_SIZE, false), // vertices
                    (Triangle::MIN_BINDING_SIZE, false),       // triangles
                    (f32::MIN_BINDING_SIZE, false),            // sdf
                ],
                immediate_size: 0,
                constants: [("WORKGROUP_SIZE", workgroup_size.get() as f64),]
            }
        );

        Self {
            triangle_sdf,
            workgroup_size,
        }
    }

    fn record(
        &self,
        context: &mut GpuContext,
        encoder: &mut CommandEncoder,
        Input {
            test_positions,
            vertices,
            triangles,
        }: Input,
        _: Parameters,
    ) -> Result<Output, GpuError> {
        let sdf = context.allocator()?.allocate::<f32>(
            "sdf",
            (test_positions.len::<Vector4<f32>>().get())
                .try_into()
                .unwrap(),
        )?;

        let mut compute_pass = encoder.begin_compute_pass(self.triangle_sdf.label);
        compute_pass.set_pipeline(&self.triangle_sdf.compute_pipeline);
        compute_pass.set_bind_group(
            0,
            &create_bind_group(
                context.device(),
                &self.triangle_sdf,
                [
                    test_positions.binding(),
                    vertices.binding(),
                    triangles.binding(),
                    sdf.binding(),
                ],
            ),
            &[],
        );
        let indirect = Indirect::new(IndirectSettings {
            workgroup_size: self.workgroup_size,
            dispatch_limit: (u16::MAX as u32).try_into().unwrap(),
            len: test_positions.len::<Vector4<f32>>().get() as u32,
        });
        compute_pass.dispatch_workgroups(indirect.x, indirect.y, indirect.z);

        Ok(Output { sdf })
    }
}
