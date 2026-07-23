// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, iter::once, num::NonZeroU32};

use squishy_volumes_xpu::Harness;

use crate::{
    Allocation, CommandEncoder, CompiledModule, ComputePass, ExceedingLimit, GpuAllocator,
    GpuAllocatorError, GpuError, GpuStatus,
};

pub struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    subgroup_size: Option<NonZeroU32>,

    status: Allocation,

    next_shader_id: u32,
    shader_id_to_label: BTreeMap<u32, &'static str>,
    shader_label_to_id: BTreeMap<&'static str, u32>,

    allocator: Option<GpuAllocator>,
    indirect_allocator: Option<GpuAllocator>,
}

fn requirements(enable_scope_profiling: bool) -> (wgpu::Features, wgpu::Limits) {
    let mut features = wgpu::Features::empty();
    features |= wgpu::Features::SUBGROUP;
    features |= wgpu::Features::IMMEDIATES;
    features |= wgpu::Features::TIMESTAMP_QUERY;
    features |= wgpu::Features::PASSTHROUGH_SHADERS;
    features |= wgpu::Features::SUBGROUP_SIZE_CONTROL;

    if enable_scope_profiling {
        features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    }

    let mut limits = wgpu::Limits::downlevel_defaults();
    limits.max_immediate_size = 4;
    limits.max_storage_buffers_per_shader_stage = 20;

    (features, limits)
}

impl GpuContext {
    pub fn available_gpus() -> Vec<String> {
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(instance_descriptor);
        pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
            .into_iter()
            .map(|adapter| adapter.get_info().name)
            .collect()
    }

    pub fn new(gpu: Option<String>) -> Result<Self, GpuError> {
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = wgpu::Backends::PRIMARY;

        let instance = wgpu::Instance::new(instance_descriptor);
        let adapter = if let Some(requested) = gpu {
            pollster::block_on(instance.enumerate_adapters(wgpu::Backends::PRIMARY))
                .into_iter()
                .find(|adapter| adapter.get_info().name == requested)
                .ok_or(GpuError::AdapterNotFound {
                    requested,
                    available: Self::available_gpus(),
                })?
        } else {
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?
        };
        tracing::info!("Running on Adapter: {:#?}", adapter.get_info());

        let wgpu::Limits {
            max_compute_workgroups_per_dimension,
            max_buffer_size,
            max_storage_buffer_binding_size,
            ..
        } = adapter.limits();

        if max_compute_workgroups_per_dimension < u16::MAX as u32 {
            return Err(GpuError::SmallMaxWorkGroupPerDimension);
        }

        let downlevel_capabilities = adapter.get_downlevel_capabilities();
        downlevel_capabilities
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
            .then_some(())
            .ok_or(GpuError::ComputeNotSupported)?;

        let enable_scope_profiling = adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
        if !enable_scope_profiling {
            tracing::warn!(
                "Missing {:?}, GPU profiling is limited.",
                wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
            )
        }

        let (required_features, mut required_limits) = requirements(enable_scope_profiling);
        required_limits.max_buffer_size = max_buffer_size;
        required_limits.max_storage_buffer_binding_size = max_storage_buffer_binding_size;
        tracing::info!(
            max_buffer_size = required_limits.max_buffer_size,
            max_storage_buffer_binding_size = required_limits.max_storage_buffer_binding_size,
        );

        let missing_features = required_features.difference(adapter.features());
        missing_features
            .is_empty()
            .then_some(())
            .ok_or(GpuError::MissingRequiredFeatures(missing_features))?;

        let mut exceeding_limits: Vec<ExceedingLimit> = Default::default();
        required_limits.check_limits_with_fail_fn(
            &adapter.limits(),
            false,
            |name, required, allowed| {
                exceeding_limits.push(ExceedingLimit {
                    name,
                    required,
                    allowed,
                });
            },
        );

        exceeding_limits
            .is_empty()
            .then_some(())
            .ok_or(GpuError::ExceedingRequiredLimits(exceeding_limits))?;

        let wgpu::AdapterInfo {
            subgroup_min_size,
            subgroup_max_size,
            ..
        } = adapter.get_info();
        let subgroup_size = if subgroup_min_size == subgroup_max_size {
            Some(
                adapter
                    .get_info()
                    .subgroup_min_size
                    .try_into()
                    .map_err(|_| GpuError::SubgroupSizeZero)?,
            )
        } else {
            tracing::warn!(
                subgroup_min_size,
                subgroup_max_size,
                "The subgroup size is variable, this might cause problems."
            );
            None
        };

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features,
                required_limits,
                experimental_features: unsafe { wgpu::ExperimentalFeatures::enabled() },
                memory_hints: wgpu::MemoryHints::MemoryUsage,
                trace: wgpu::Trace::Off,
            }))?;

        let status = Allocation::new(&device, "status", &[GpuStatus::default()])?;

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            subgroup_size,

            status,

            next_shader_id: 1,
            shader_id_to_label: Default::default(),
            shader_label_to_id: Default::default(),

            allocator: None,
            indirect_allocator: None,
        })
    }

    pub fn instance(&self) -> &wgpu::Instance {
        &self.instance
    }

    pub fn adapter(&self) -> &wgpu::Adapter {
        &self.adapter
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn subgroup_size(&self) -> Option<NonZeroU32> {
        self.subgroup_size
    }

    pub fn setup_allocator(
        &mut self,
        harness: Option<&Harness>,
        size: u64,
        label: &'static str,
        scram: bool,
    ) -> Result<(), GpuError> {
        self.allocator = Some(GpuAllocator::new(self, harness, size, label, scram)?);
        Ok(())
    }

    pub fn resize_allocator(&mut self, size: u64, scram: bool) -> Result<(), GpuError> {
        self.allocator
            .as_mut()
            .ok_or(GpuError::AllocatorMissing)?
            .resize_to(&self.device, size, scram)?;
        Ok(())
    }

    pub fn allocator(&mut self) -> Result<&mut GpuAllocator, GpuError> {
        self.allocator.as_mut().ok_or(GpuError::AllocatorMissing)
    }

    pub fn setup_indirect_allocator(
        &mut self,
        size: u64,
        label: &'static str,
        scram: bool,
    ) -> Result<(), GpuError> {
        self.indirect_allocator = Some(GpuAllocator::new(self, None, size, label, scram)?);
        Ok(())
    }

    pub fn indirect_allocator(&mut self) -> Result<&mut GpuAllocator, GpuError> {
        self.indirect_allocator
            .as_mut()
            .ok_or(GpuError::IndirectAllocatorMissing)
    }

    pub fn enter_module<'a>(
        &'a self,
        encoder: &'a mut CommandEncoder,
        compiled_module: &CompiledModule,
        entries: impl IntoIterator<Item = wgpu::BufferBinding<'a>>,
    ) -> ComputePass<'a> {
        let status_binding = self.status.binding();
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: compiled_module.label,
            layout: &compiled_module.bind_group_layout,
            entries: &entries
                .into_iter()
                .chain(once(status_binding))
                .enumerate()
                .map(|(binding, entry)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: wgpu::BindingResource::Buffer(entry),
                })
                .collect::<Vec<_>>(),
        });

        let mut compute_pass = encoder.begin_compute_pass(compiled_module.label);
        compute_pass.set_pipeline(&compiled_module.compute_pipeline);
        compute_pass.set_bind_group(0, &bind_group, &[]);

        compute_pass
    }

    pub fn status(&self) -> Allocation {
        self.status.clone()
    }

    pub fn reset_status(&mut self) -> Result<(), GpuAllocatorError> {
        self.status = Allocation::new(&self.device, "status", &[GpuStatus::default()])?;
        Ok(())
    }

    pub fn get_shader_id(&mut self, label: &'static str) -> u32 {
        let shader_id = self
            .shader_label_to_id
            .entry(label)
            .or_insert_with(|| self.next_shader_id);
        self.shader_id_to_label.insert(*shader_id, label);

        self.next_shader_id += 1;
        *shader_id
    }

    pub fn get_shader_label(&self, id: u32) -> Option<&'static str> {
        self.shader_id_to_label.get(&id).cloned()
    }
}
