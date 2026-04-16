// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use rand::prelude::*;
use rand::rngs::ChaCha8Rng;
use std::{
    mem::take,
    num::NonZeroU64,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use super::*;

use num::integer::lcm;
use wgpu::util::DeviceExt as _;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum GpuAllocatorError {
    #[error("Failed to allocate buffer, requested {requested}, but max is {max}")]
    ExceedingMaxBufferSize { requested: u64, max: u64 },
    #[error("Failed to allocate buffer binding, requested {requested}, but max is {max}")]
    ExceedingMaxBufferBindingSize { requested: u64, max: u64 },
    #[error(
        "Failed to allocate binding, requested {requested},
but the largest continous free space is {continuous_free}.
In total free: {total_free}"
    )]
    FailedToFindSpace {
        requested: u64,
        continuous_free: u64,
        total_free: u64,
    },
}

pub struct GpuAllocator {
    min_storage_buffer_offset_alignment: u64,
    max_storage_buffer_binding_size: u64,
    buffer: Arc<wgpu::Buffer>,
    partitions: Vec<Partition>,
}

#[derive(Clone)]
pub struct Allocation {
    label: &'static str,
    buffer: Arc<wgpu::Buffer>,
    partition: Partition,
}

impl Allocation {
    pub fn new<T: AllowedInBinding + bytemuck::Pod>(
        device: &wgpu::Device,
        label: &'static str,
        contents: &[T],
    ) -> Self {
        let buffer = Arc::new(
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(contents),
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::INDIRECT,
            }),
        );
        let partition = Partition {
            start: 0,
            end: buffer.size(),
            used: Arc::new(true.into()),
        };
        Self {
            label,
            buffer,
            partition,
        }
    }

    pub fn label(&self) -> &'static str {
        self.label
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn offset(&self) -> u64 {
        self.partition.start
    }

    pub fn binding<'a>(&'a self) -> wgpu::BufferBinding<'a> {
        let buffer = &self.buffer;
        let offset = self.partition.start;
        assert!(self.partition.end > self.partition.start);
        let size = Some(NonZeroU64::new(self.partition.end - self.partition.start).unwrap());
        wgpu::BufferBinding {
            buffer,
            offset,
            size,
        }
    }

    pub fn len<T: AllowedInBinding>(&self) -> NonZeroU64 {
        let size = self.size().get();
        assert!(size.is_multiple_of(T::MIN_BINDING_SIZE.get()));
        NonZeroU64::new(size / T::MIN_BINDING_SIZE.get()).unwrap()
    }

    pub fn size(&self) -> NonZeroU64 {
        assert!(self.partition.end > self.partition.start);
        NonZeroU64::new(self.partition.end - self.partition.start).unwrap()
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        self.partition.used.store(false, Ordering::Relaxed);
    }
}

impl GpuAllocator {
    pub fn new(
        context: &GpuContext,
        size: u64,
        label: &'static str,
        scram: bool,
    ) -> Result<Self, GpuAllocatorError> {
        if context.adapter().limits().max_buffer_size < size {
            return Err(GpuAllocatorError::ExceedingMaxBufferSize {
                requested: size,
                max: context.adapter().limits().max_buffer_size,
            });
        }

        let buffer = Arc::new(if scram {
            let random_content: Vec<u8> = ChaCha8Rng::seed_from_u64(42)
                .random_iter::<u8>()
                .take(size as usize)
                .collect();
            context
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents: &random_content,
                    usage: wgpu::BufferUsages::STORAGE
                        | wgpu::BufferUsages::COPY_DST
                        | wgpu::BufferUsages::COPY_SRC
                        | wgpu::BufferUsages::INDIRECT,
                })
        } else {
            context.device().create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC
                    | wgpu::BufferUsages::INDIRECT,
                mapped_at_creation: false,
            })
        });

        let partitions = vec![Partition {
            start: 0,
            end: size,
            used: Arc::new(false.into()),
        }];

        let min_storage_buffer_offset_alignment = context
            .device()
            .limits()
            .min_storage_buffer_offset_alignment
            as u64;
        let max_storage_buffer_binding_size =
            context.device().limits().max_storage_buffer_binding_size;

        Ok(Self {
            min_storage_buffer_offset_alignment,
            max_storage_buffer_binding_size,
            buffer,
            partitions,
        })
    }

    pub fn allocate<T: AllowedInBinding>(
        &mut self,
        label: &'static str,
        len: NonZeroU64,
    ) -> Result<Allocation, GpuAllocatorError> {
        self.allocate_raw(
            label,
            NonZeroU64::new(T::MIN_BINDING_SIZE.get() * len.get()).unwrap(),
            T::ALIGNMENT,
        )
    }

    pub fn allocate_raw(
        &mut self,
        label: &'static str,
        size: NonZeroU64,
        align: NonZeroU64,
    ) -> Result<Allocation, GpuAllocatorError> {
        // sanity check
        if self.max_storage_buffer_binding_size < size.get() {
            return Err(GpuAllocatorError::ExceedingMaxBufferBindingSize {
                requested: size.get(),
                max: self.max_storage_buffer_binding_size,
            });
        }

        self.fix();

        let mut allocation = None;
        for partition in take(&mut self.partitions) {
            // already have allocation or in use
            if allocation.is_some() || partition.used.load(Ordering::Relaxed) {
                self.partitions.push(partition);
                continue;
            }

            let align = lcm(align.get(), self.min_storage_buffer_offset_alignment);
            let aligned_start = partition.start.next_multiple_of(align);

            // does it fit?
            if partition.end < aligned_start + size.get() {
                self.partitions.push(partition);
                continue;
            }

            // alignment padding goes back
            self.partitions.push(Partition {
                start: partition.start,
                end: aligned_start,
                used: Arc::new(false.into()),
            });

            // this part is actually used
            // we also return a copy
            let allocated_partition = Partition {
                start: aligned_start,
                end: aligned_start + size.get(),
                used: Arc::new(true.into()),
            };
            allocation = Some(Allocation {
                label,
                buffer: self.buffer.clone(),
                partition: allocated_partition.clone(),
            });
            self.partitions.push(allocated_partition);

            // leftovers
            self.partitions.push(Partition {
                start: aligned_start + size.get(),
                end: partition.end,
                used: Arc::new(false.into()),
            });
        }

        allocation.ok_or(GpuAllocatorError::FailedToFindSpace {
            requested: size.get(),
            continuous_free: self.biggest_free(),
            total_free: self.total_free(),
        })
    }

    fn free_sizes(&self) -> impl Iterator<Item = u64> + '_ {
        self.partitions
            .iter()
            .map(|Partition { start, end, used }| {
                if used.load(Ordering::Relaxed) {
                    0
                } else {
                    end - start
                }
            })
    }

    pub fn biggest_free(&self) -> u64 {
        self.free_sizes().max().unwrap_or(0)
    }

    pub fn total_free(&self) -> u64 {
        self.free_sizes().sum()
    }

    pub fn fix(&mut self) {
        // track the current free part
        let mut free = None;
        for partition in take(&mut self.partitions) {
            // forget about empty partitions
            if partition.start == partition.end {
                continue;
            }

            // this partition is in use
            // push the free part and itself
            if partition.used.load(Ordering::Relaxed) {
                if let Some(free) = free.take() {
                    self.partitions.push(free);
                }
                self.partitions.push(partition);
                continue;
            }

            // if there's no free part being tracked, start one
            let Some(free) = &mut free else {
                free = Some(partition);
                continue;
            };

            // extend it
            free.end = partition.end;
        }

        // put the last piece in
        if let Some(free) = free.take() {
            self.partitions.push(free);
        }
    }
}

#[derive(Clone)]
struct Partition {
    start: u64,
    end: u64,
    used: Arc<AtomicBool>,
}

#[macro_export]
macro_rules! let_allocation_init {
    ($device:expr, $name:ident($contents:expr)) => {
        let $name = Allocation::new($device, stringify!($name), $contents);
    };
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;

    #[test]
    fn test_simple() {
        let aligned = NonZeroU64::new(256).unwrap();
        let binding_size = aligned.get() * u32::MIN_BINDING_SIZE.get();
        let size = binding_size * 4;

        let context = SHARED_CONTEXT.lock().unwrap();
        let mut allocator = GpuAllocator::new(&context, size, "allocation", true).unwrap();

        let _a = allocator.allocate::<u32>("a", aligned).unwrap();
        let _b = allocator.allocate::<u32>("b", aligned).unwrap();
        let _c = allocator.allocate::<u32>("c", aligned).unwrap();
        let _d = allocator.allocate::<u32>("d", aligned).unwrap();

        allocator.fix();
        assert_eq!(allocator.total_free(), 0);
        assert_eq!(allocator.biggest_free(), 0);

        drop(_a);
        allocator.fix();
        assert_eq!(allocator.total_free(), binding_size);
        assert_eq!(allocator.biggest_free(), binding_size);

        drop(_c);
        allocator.fix();
        assert_eq!(allocator.total_free(), binding_size * 2);
        assert_eq!(allocator.biggest_free(), binding_size);

        drop(_d);
        allocator.fix();
        assert_eq!(allocator.total_free(), binding_size * 3);
        assert_eq!(allocator.biggest_free(), binding_size * 2);
    }

    #[test]
    fn test_buffer_too_large() {
        let context = SHARED_CONTEXT.lock().unwrap();
        assert!(matches!(
            GpuAllocator::new(&context, u64::MAX, "allocation", false),
            Err(GpuAllocatorError::ExceedingMaxBufferSize { .. })
        ));
    }

    #[test]
    fn test_buffer_binding_too_large() {
        let context = SHARED_CONTEXT.lock().unwrap();
        let mut allocator = GpuAllocator::new(&context, 42, "allocation", false).unwrap();
        assert!(matches!(
            allocator.allocate_raw(
                "too large",
                NonZeroU64::new(u64::MAX).unwrap(),
                NonZeroU64::new(4).unwrap(),
            ),
            Err(GpuAllocatorError::ExceedingMaxBufferBindingSize { .. })
        ));
    }

    #[test]
    fn test_no_space() {
        let aligned = NonZeroU64::new(256).unwrap();
        let double_aligned = NonZeroU64::new(256 * 2).unwrap();
        let binding_size = aligned.get() * u32::MIN_BINDING_SIZE.get();
        let size = binding_size * 4;

        let context = SHARED_CONTEXT.lock().unwrap();
        let mut allocator = GpuAllocator::new(&context, size, "allocation", false).unwrap();

        let _a = allocator.allocate::<u32>("a", aligned).unwrap();
        let _b = allocator.allocate::<u32>("b", aligned).unwrap();
        let _c = allocator.allocate::<u32>("c", aligned).unwrap();

        drop(_b);

        assert!(matches!(
            allocator.allocate::<u32>("no space", double_aligned),
            Err(GpuAllocatorError::FailedToFindSpace { .. }),
        ));
    }
}
