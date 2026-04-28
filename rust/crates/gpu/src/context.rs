// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU32;

use crate::{ExceedingLimit, GpuAllocator, GpuError};

pub struct GpuContext {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,

    subgroup_size: NonZeroU32,

    allocator: Option<GpuAllocator>,
    indirect_allocator: Option<GpuAllocator>,
}

fn requirements(max_num_particles: u32) -> (wgpu::Features, wgpu::Limits) {
    let mut features = wgpu::Features::empty();
    features |= wgpu::Features::SUBGROUP;
    features |= wgpu::Features::IMMEDIATES;
    features |= wgpu::Features::TIMESTAMP_QUERY;

    let mut limits = wgpu::Limits::downlevel_defaults();
    limits.max_immediate_size = 4;
    limits.max_storage_buffers_per_shader_stage = 18;

    let size_requirement = (max_num_particles * 1024) as u64;

    limits.max_buffer_size = size_requirement;
    limits.max_storage_buffer_binding_size = size_requirement;

    (features, limits)
}

impl GpuContext {
    pub fn new(max_num_particles: u32) -> Result<Self, GpuError> {
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

        let (required_features, required_limits) = requirements(max_num_particles);

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

        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            subgroup_size,
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
    ) -> Result<(), GpuError> {
        self.allocator = Some(GpuAllocator::new(self, size, label, scram)?);
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
}
