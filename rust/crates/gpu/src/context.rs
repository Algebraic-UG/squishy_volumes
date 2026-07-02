// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{collections::BTreeMap, iter::once, num::NonZeroU32};

use crate::{
    Allocation, CommandEncoder, CompiledModule, ComputePass, ExceedingLimit, GpuAllocator,
    GpuAllocatorError, GpuError, GpuStatus,
};

pub struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    subgroup_size: NonZeroU32,

    status: Allocation,

    next_shader_id: u32,
    shader_id_to_label: BTreeMap<u32, &'static str>,
    shader_label_to_id: BTreeMap<&'static str, u32>,

    allocator: Option<GpuAllocator>,
    indirect_allocator: Option<GpuAllocator>,
}

fn requirements() -> (wgpu::Features, wgpu::Limits) {
    let mut features = wgpu::Features::empty();
    features |= wgpu::Features::SUBGROUP;
    features |= wgpu::Features::IMMEDIATES;
    features |= wgpu::Features::TIMESTAMP_QUERY;
    features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;

    let mut limits = wgpu::Limits::downlevel_defaults();
    limits.max_immediate_size = 4;
    limits.max_storage_buffers_per_shader_stage = 20;

    (features, limits)
}

impl GpuContext {
    pub fn new() -> Result<Self, GpuError> {
        let mut instance_descriptor = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_descriptor.backends = wgpu::Backends::PRIMARY;

        let instance = wgpu::Instance::new(instance_descriptor);
        let adapter =
            pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))?;
        tracing::info!("Running on Adapter: {:#?}", adapter.get_info());

        if adapter.limits().max_compute_workgroups_per_dimension < u16::MAX as u32 {
            return Err(GpuError::SmallMaxWorkGroupPerDimension);
        }
        if adapter.get_info().subgroup_min_size != adapter.get_info().subgroup_min_size {
            return Err(GpuError::VariableSubgroupSize);
        }
        let subgroup_size = adapter
            .get_info()
            .subgroup_min_size
            .try_into()
            .map_err(|_| GpuError::SubgroupSizeZero)?;

        let downlevel_capabilities = adapter.get_downlevel_capabilities();
        downlevel_capabilities
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS)
            .then_some(())
            .ok_or(GpuError::ComputeNotSupported)?;

        let (required_features, mut required_limits) = requirements();
        required_limits.max_buffer_size = adapter.limits().max_buffer_size;
        required_limits.max_storage_buffer_binding_size =
            adapter.limits().max_storage_buffer_binding_size;
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

        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features,
                required_limits,
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
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

    pub fn subgroup_size(&self) -> NonZeroU32 {
        self.subgroup_size
    }

    pub fn setup_allocator(
        &mut self,
        size: u64,
        label: &'static str,
        scram: bool,
    ) -> Result<(), GpuAllocatorError> {
        self.allocator = Some(GpuAllocator::new(self, size, label, scram)?);
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
        self.indirect_allocator = Some(GpuAllocator::new(self, size, label, scram)?);
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
