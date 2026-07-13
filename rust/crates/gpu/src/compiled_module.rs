// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{iter, num::NonZeroU64};

use crate::{AllowedInBinding, GpuContext, GpuStatus};

pub struct CompiledModule {
    pub label: Option<&'static str>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
}

pub struct CompiledModuleSettings<'a, BindGroupEntries, Constants> {
    pub context: &'a mut GpuContext,
    pub bind_group_entries: BindGroupEntries,
    pub immediate_size: u32,
    pub constants: Constants,
}

impl CompiledModule {
    pub fn new<BindGroupEntries, Constants>(
        label: &'static str,
        shader_module_descriptor: wgpu::ShaderModuleDescriptor,
        CompiledModuleSettings {
            context,
            bind_group_entries,
            immediate_size,
            constants,
        }: CompiledModuleSettings<BindGroupEntries, Constants>,
    ) -> Self
    where
        BindGroupEntries: IntoIterator<Item = (NonZeroU64, bool)>,
        Constants: IntoIterator<Item = (&'static str, f64)>,
    {
        let shader_id = context.get_shader_id(label);
        let label = Some(label);
        let device = context.device();
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label,
            entries: &bind_group_entries
                .into_iter()
                .chain(iter::once((GpuStatus::MIN_BINDING_SIZE, false)))
                .enumerate()
                .map(
                    |(binding, (min_binding_size, read_only))| wgpu::BindGroupLayoutEntry {
                        binding: binding as u32,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only },
                            min_binding_size: Some(min_binding_size),
                            has_dynamic_offset: false,
                        },
                        count: None,
                    },
                )
                .collect::<Vec<_>>(),
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label,
                    bind_group_layouts: &[Some(&bind_group_layout)],
                    immediate_size,
                }),
            ),
            module: &device.create_shader_module(shader_module_descriptor),
            entry_point: Some("main"),
            compilation_options: wgpu::PipelineCompilationOptions {
                constants: &constants
                    .into_iter()
                    .chain(iter::once(("SHADER_ID", shader_id as f64)))
                    .collect::<Vec<_>>(),
                ..Default::default()
            },
            cache: None,
        });

        CompiledModule {
            label,
            bind_group_layout,
            compute_pipeline,
        }
    }
}

#[macro_export]
macro_rules! let_compiled_module {
    ($name:ident, $settings:expr) => {
        let $name = CompiledModule::new(
            stringify!($name),
            wgpu::ShaderModuleDescriptor {
                label: Some(stringify!($name)),
                source: wgpu::ShaderSource::Wgsl(
                    include_str!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".wgsl")).into(),
                ),
            },
            $settings,
        );
    };
}
