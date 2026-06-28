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
    ops::Range,
    sync::{Arc, Weak},
};

use super::*;

use num::integer::lcm;
use wgpu::util::DeviceExt as _;

use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum GpuAllocatorError {
    #[error("Failed to allocate buffer, requested {requested}, but max is {max}")]
    ExceedingMaxBufferSize { requested: u64, max: u64 },
    #[error(
        "Failed to allocate buffer binding for {label}, requested {requested}, but max is {max}"
    )]
    ExceedingMaxBufferBindingSize {
        label: &'static str,
        requested: u64,
        max: u64,
    },
    #[error(
        "Failed to allocate binding for {label}, requested {requested},
but the largest continous free space is {continuous_free}.
In total free: {total_free}"
    )]
    FailedToFindSpace {
        label: &'static str,
        requested: u64,
        continuous_free: u64,
        total_free: u64,
    },
    #[error("Expected non-zero allocation for {label}")]
    AllocationEmpty { label: &'static str },
}

pub struct GpuAllocator {
    max_storage_buffer_binding_size: u64,
    buffers: Vec<GpuBuffer>,
}

struct GpuBuffer {
    min_storage_buffer_offset_alignment: u64,
    buffer: Arc<wgpu::Buffer>,
    partitions: Vec<Partition>,
}

#[derive(Clone)]
pub struct Allocation {
    label: &'static str,
    buffer: Arc<wgpu::Buffer>,
    range: Range<u64>,
    _keep_alive: Arc<()>,
}

impl Allocation {
    pub fn new<T: AllowedInBinding + bytemuck::Pod>(
        device: &wgpu::Device,
        label: &'static str,
        contents: &[T],
    ) -> Result<Self, GpuAllocatorError> {
        if contents.is_empty() {
            return Err(GpuAllocatorError::AllocationEmpty { label });
        }

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
        let (partition, _keep_alive) = Partition::new(0..buffer.size());

        Ok(Self {
            label,
            buffer,
            range: partition.range,
            _keep_alive,
        })
    }

    pub fn label(&self) -> &'static str {
        self.label
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn offset(&self) -> u64 {
        self.range.start
    }

    pub fn binding<'a>(&'a self) -> wgpu::BufferBinding<'a> {
        let buffer = &self.buffer;
        let offset = self.range.start;
        let size = Some(self.size());
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
        assert!(self.range.end >= self.range.start);
        NonZeroU64::new(self.range.end - self.range.start).unwrap()
    }
}

impl GpuBuffer {
    fn new(
        context: &GpuContext,
        size: u64,
        label: &str,
        scram: bool,
    ) -> Result<Self, GpuAllocatorError> {
        tracing::info!(label, size, "creating new gpu buffer");
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

        let partitions = vec![Partition::unused(0..size)];

        let min_storage_buffer_offset_alignment = context
            .device()
            .limits()
            .min_storage_buffer_offset_alignment
            as u64;

        Ok(Self {
            min_storage_buffer_offset_alignment,
            buffer,
            partitions,
        })
    }

    fn allocate_raw(
        &mut self,
        label: &'static str,
        size: NonZeroU64,
        align: NonZeroU64,
    ) -> Option<Allocation> {
        self.fix();

        let mut allocation = None;
        for partition in take(&mut self.partitions) {
            // already have allocation or in use
            if allocation.is_some() || partition.in_use() {
                self.partitions.push(partition);
                continue;
            }

            let align = lcm(align.get(), self.min_storage_buffer_offset_alignment);
            let aligned_start = partition.range.start.next_multiple_of(align);

            // does it fit?
            if partition.range.end < aligned_start + size.get() {
                self.partitions.push(partition);
                continue;
            }

            // alignment padding goes back
            self.partitions
                .push(Partition::unused(partition.range.start..aligned_start));

            // this part is actually used
            // we keep the arc alive
            let (allocated_partition, _keep_alive) =
                Partition::new(aligned_start..aligned_start + size.get());
            allocation = Some(Allocation {
                label,
                buffer: self.buffer.clone(),
                range: allocated_partition.range.clone(),
                _keep_alive,
            });
            self.partitions.push(allocated_partition);

            // leftovers
            self.partitions.push(Partition::unused(
                aligned_start + size.get()..partition.range.end,
            ));
        }

        allocation
    }

    fn free_sizes(&self) -> impl Iterator<Item = u64> + '_ {
        self.partitions.iter().map(|partition| {
            if partition.in_use() {
                0
            } else {
                partition.size()
            }
        })
    }

    fn biggest_free(&self) -> u64 {
        self.free_sizes().max().unwrap_or(0)
    }

    fn total_free(&self) -> u64 {
        self.free_sizes().sum()
    }

    fn fix(&mut self) {
        // track the current free part
        let mut free = None;
        for partition in take(&mut self.partitions) {
            // forget about empty partitions
            if partition.is_empty() {
                continue;
            }

            // this partition is in use
            // push the free part and itself
            if partition.in_use() {
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
            free.range.end = partition.range.end;
        }

        // put the last piece in
        if let Some(free) = free.take() {
            self.partitions.push(free);
        }
    }

    fn check_overlap(&self) {
        for a in 0..self.partitions.len() {
            for b in a + 1..self.partitions.len() {
                println!("{a} vs {b}");
                let a = &self.partitions[a];
                let b = &self.partitions[b];
                println!("{:?} vs {:?}", a.range, b.range);
                assert!(!a.overlap(b));
            }
        }
    }
}

impl GpuAllocator {
    pub fn new(
        context: &GpuContext,
        size: u64,
        label: &'static str,
        scram: bool,
    ) -> Result<Self, GpuAllocatorError> {
        let max_buffer_size = context.adapter().limits().max_buffer_size;
        let num_buffer = size.div_ceil(max_buffer_size);
        tracing::info!(label, size, num_buffer, "creating new gpu allocator");

        let max_storage_buffer_binding_size =
            context.device().limits().max_storage_buffer_binding_size;

        Ok(Self {
            max_storage_buffer_binding_size,
            buffers: (0..num_buffer)
                .map(|i| {
                    assert!(size > i * max_buffer_size);
                    let size = (size - i * max_buffer_size).min(max_buffer_size);
                    GpuBuffer::new(context, size, &format!("{label}-{i}"), scram)
                })
                .collect::<Result<_, _>>()?,
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
                label,
                requested: size.get(),
                max: self.max_storage_buffer_binding_size,
            });
        }

        for buffer in &mut self.buffers {
            if let Some(allocation) = buffer.allocate_raw(label, size, align) {
                return Ok(allocation);
            }
        }

        Err(GpuAllocatorError::FailedToFindSpace {
            label,
            requested: size.get(),
            continuous_free: self.biggest_free(),
            total_free: self.total_free(),
        })
    }

    pub fn biggest_free(&self) -> u64 {
        self.buffers
            .iter()
            .map(GpuBuffer::biggest_free)
            .max()
            .unwrap_or(0)
    }

    pub fn total_free(&self) -> u64 {
        self.buffers.iter().map(GpuBuffer::total_free).sum()
    }

    pub fn check_overlap(&self) {
        self.buffers.iter().for_each(GpuBuffer::check_overlap);
    }

    fn fix(&mut self) {
        self.buffers.iter_mut().for_each(GpuBuffer::fix);
    }
}

#[derive(Clone)]
struct Partition {
    range: Range<u64>,
    counter: Weak<()>,
}

impl Partition {
    // immediately drop the arc
    fn unused(range: Range<u64>) -> Self {
        Self::new(range).0
    }

    fn new(range: Range<u64>) -> (Self, Arc<()>) {
        let arc = Arc::new(());
        (
            Self {
                range,
                counter: Arc::downgrade(&arc),
            },
            arc,
        )
    }

    fn in_use(&self) -> bool {
        Weak::strong_count(&self.counter) > 0
    }

    fn size(&self) -> u64 {
        assert!(self.range.end >= self.range.start);
        self.range.end - self.range.start
    }

    fn is_empty(&self) -> bool {
        self.range.is_empty()
    }

    fn overlap(&self, other: &Self) -> bool {
        if self.is_empty() || other.is_empty() {
            return false;
        }
        self.range.contains(&other.range.start) || other.range.contains(&self.range.start)
    }
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
