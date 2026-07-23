// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    iter,
    num::{NonZeroU32, NonZeroU64},
};

use crate::{AllowedInBinding, GpuContext, GpuPipelineCreationError, GpuStatus};

pub struct CompiledModule {
    pub label: Option<&'static str>,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub compute_pipeline: wgpu::ComputePipeline,
    pub subgroup_size: NonZeroU32,
}

pub struct CompiledModuleSettings<'a, BindGroupEntries, Constants> {
    pub context: &'a mut GpuContext,
    pub workgroup_size: NonZeroU32,
    pub bind_group_entries: BindGroupEntries,
    pub immediate_size: u32,
    pub constants: Constants,
}

impl CompiledModule {
    pub fn new<BindGroupEntries, Constants>(
        label_raw: &'static str,
        wgsl_source: &'static str,
        CompiledModuleSettings {
            context,
            workgroup_size,
            bind_group_entries,
            immediate_size,
            constants,
        }: CompiledModuleSettings<BindGroupEntries, Constants>,
    ) -> Result<Self, GpuPipelineCreationError>
    where
        BindGroupEntries: IntoIterator<Item = (NonZeroU64, bool)>,
        Constants: IntoIterator<Item = (&'static str, f64)>,
        <Constants as IntoIterator>::IntoIter: Clone,
    {
        let shader_id = context.get_shader_id(label_raw);
        let constants = constants.into_iter().chain([
            ("SHADER_ID", shader_id as f64),
            ("WORKGROUP_SIZE", workgroup_size.get() as f64),
        ]);
        {
            let mut constant_names = std::collections::BTreeSet::default();
            for (name, _) in constants.clone() {
                if !constant_names.insert(name) {
                    return Err(GpuPipelineCreationError::PipelineDuplicateConstant(name));
                }
            }
        }
        let label = Some(label_raw);
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

        let layout = Some(
            &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label,
                bind_group_layouts: &[Some(&bind_group_layout)],
                immediate_size,
            }),
        );

        let fix_subgroup_size = context.subgroup_size().is_none()
            && context.adapter().get_info().backend == wgpu::Backend::Vulkan;

        let subgroup_size: NonZeroU32;
        let compute_pipeline;
        if fix_subgroup_size {
            subgroup_size = context
                .adapter()
                .get_info()
                .subgroup_max_size
                .try_into()
                .unwrap();
            compute_pipeline = compute_pipeline_with_fixed_subgroup_size(
                context,
                label,
                layout,
                wgsl_source,
                workgroup_size,
                subgroup_size,
                constants,
            )?;
        } else {
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label,
                source: wgpu::ShaderSource::Wgsl(wgsl_source.into()),
            });

            compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label,
                layout,
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants.collect::<Vec<_>>(),
                    ..Default::default()
                },
                cache: None,
            });

            subgroup_size = context
                .subgroup_size()
                .or_else(|| {
                    compute_pipeline
                        .get_sub_group_size()
                        .and_then(|subgroup_size| (subgroup_size as u32).try_into().ok())
                })
                .ok_or(GpuPipelineCreationError::FailedToDetermineSubgroupSize {
                    label: label_raw,
                })?;
        };

        Ok(CompiledModule {
            label,
            bind_group_layout,
            compute_pipeline,
            subgroup_size,
        })
    }

    pub fn check_same_sugroup_size(&self, other: &Self) -> Result<(), GpuPipelineCreationError> {
        if self.subgroup_size != other.subgroup_size {
            Err(GpuPipelineCreationError::SubgroupSizeMismatch {
                a_label: self.label.unwrap_or("unlabeled"),
                a_size: self.subgroup_size.get(),
                b_label: other.label.unwrap_or("unlabeled"),
                b_size: other.subgroup_size.get(),
            })
        } else {
            Ok(())
        }
    }

    pub fn check_workgroup_size_multiple_of_subgroup_size(
        &self,
        workgroup_size: u32,
    ) -> Result<(), GpuPipelineCreationError> {
        if !workgroup_size.is_multiple_of(self.subgroup_size.get()) {
            Err(
                GpuPipelineCreationError::WorkgroupSizeNotMultipleOfSubgroupSize {
                    label: self.label.unwrap_or("unlabeled"),
                    workgroup_size,
                    subgroup_size: self.subgroup_size.get(),
                },
            )
        } else {
            Ok(())
        }
    }

    pub fn check_subgroup_size_at_least(
        &self,
        needed: u32,
    ) -> Result<(), GpuPipelineCreationError> {
        if self.subgroup_size.get() < needed {
            Err(GpuPipelineCreationError::SubgroupSizeTooSmall {
                label: self.label.unwrap_or("unlabeled"),
                subgroup_size: self.subgroup_size.get(),
                needed,
            })
        } else {
            Ok(())
        }
    }
}

#[macro_export]
macro_rules! let_compiled_module {
    ($name:ident, $settings:expr) => {
        let $name = CompiledModule::new(
            stringify!($name),
            include_str!(concat!(env!("OUT_DIR"), "/", stringify!($name), ".wgsl")),
            $settings,
        )?;
    };
}

fn compute_pipeline_with_fixed_subgroup_size(
    context: &mut GpuContext,
    label: Option<&str>,
    layout: Option<&wgpu::PipelineLayout>,
    wgsl_source: &str,
    workgroup_size: NonZeroU32,
    subgroup_size: NonZeroU32,
    constants: impl Iterator<Item = (&'static str, f64)>,
) -> Result<wgpu::ComputePipeline, GpuPipelineCreationError> {
    #[cfg(target_os = "macos")]
    panic!();
    #[cfg(not(target_os = "macos"))]
    {
        // with the current state of wgpu the only way to fix the subgroup size is to go via passthrough shaders.
        // So we do the transpiling to spir-v now

        let module = wgpu::naga::front::wgsl::parse_str(wgsl_source).unwrap();

        // XXX: Compare with the ones we check in GpuContext cration
        let capabilities =
            wgpu::naga::valid::Capabilities::IMMEDIATES | wgpu::naga::valid::Capabilities::SUBGROUP;
        let module_info = wgpu::naga::valid::Validator::new(
            wgpu::naga::valid::ValidationFlags::all(),
            capabilities,
        )
        .validate(&module)
        .unwrap();

        // we have to do this because naga doesn't support transpiling overrides from wgsl to spir-v
        let (module, module_info) = wgpu::naga::back::pipeline_constants::process_overrides(
            &module,
            &module_info,
            None,
            &constants.map(|(s, f)| (s.to_string(), f)).collect(),
        )?;

        let options = wgpu::naga::back::spv::Options::default();
        let pipeline_options = wgpu::naga::back::spv::PipelineOptions {
            entry_point: "main".to_string(),
            shader_stage: wgpu::naga::ShaderStage::Compute,
        };

        let spv_words = wgpu::naga::back::spv::write_vec(
            &module,
            &module_info,
            &options,
            Some(&pipeline_options),
        )
        .unwrap();
        let spirv = Some(spv_words.as_slice().into());

        let workgroup_size = (workgroup_size.get(), 1, 1);

        // This is what this is all about
        let subgroup_size = wgpu::SubgroupSize::Fixed(subgroup_size.get());

        let entry_points = vec![wgpu::PassthroughShaderEntryPoint {
            name: "main".into(),
            workgroup_size,
            subgroup_size,
        }]
        .into();
        let module_descriptor = wgpu::ShaderModuleDescriptorPassthrough {
            label,
            entry_points,
            spirv,
            ..Default::default()
        };
        let module = unsafe {
            context
                .device()
                .create_shader_module_passthrough(module_descriptor)
        };

        Ok(context
            .device()
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label,
                layout,
                module: &module,
                entry_point: Some("main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                cache: None,
            }))
    }
}
