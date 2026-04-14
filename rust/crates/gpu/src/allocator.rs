// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{cell::Cell, mem::take, num::NonZeroU64, rc::Rc};

use super::*;

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
    max_storage_buffer_binding_size: u64,
    buffer: Rc<wgpu::Buffer>,
    partitions: Vec<Partition>,
}

pub struct Allocation {
    buffer: Rc<wgpu::Buffer>,
    partition: Partition,
}

impl Allocation {
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
}

impl Drop for Allocation {
    fn drop(&mut self) {
        self.partition.used.set(false);
    }
}

impl GpuAllocator {
    pub fn new(
        context: &GpuContext,
        size: u64,
        label: &'static str,
    ) -> Result<Self, GpuAllocatorError> {
        if context.adapter().limits().max_buffer_size < size {
            return Err(GpuAllocatorError::ExceedingMaxBufferSize {
                requested: size,
                max: context.adapter().limits().max_buffer_size,
            });
        }

        let buffer = Rc::new(context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        }));

        let partitions = vec![Partition {
            start: 0,
            end: size,
            used: Rc::new(Cell::new(false)),
        }];

        let max_storage_buffer_binding_size =
            context.adapter().limits().max_storage_buffer_binding_size;

        Ok(Self {
            max_storage_buffer_binding_size,
            buffer,
            partitions,
        })
    }

    pub fn allocate<T: AllowedInBinding>(
        &mut self,
        len: NonZeroU64,
    ) -> Result<Allocation, GpuAllocatorError> {
        self.allocate_raw(
            NonZeroU64::new(T::MIN_BINDING_SIZE.get() * len.get()).unwrap(),
            T::ALIGNMENT,
        )
    }

    pub fn allocate_raw(
        &mut self,
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
            if allocation.is_some() || partition.used.get() {
                self.partitions.push(partition);
                continue;
            }

            let aligned_start = partition.start.next_multiple_of(align.get());

            // does it fit?
            if partition.end < aligned_start + size.get() {
                self.partitions.push(partition);
                continue;
            }

            // alignment padding goes back
            self.partitions.push(Partition {
                start: partition.start,
                end: aligned_start,
                used: Rc::new(Cell::new(false)),
            });

            // this part is actually used
            // we also return a copy
            let allocated_partition = Partition {
                start: aligned_start,
                end: aligned_start + size.get(),
                used: Rc::new(Cell::new(true)),
            };
            allocation = Some(Allocation {
                buffer: self.buffer.clone(),
                partition: allocated_partition.clone(),
            });
            self.partitions.push(allocated_partition);

            // leftovers
            self.partitions.push(Partition {
                start: aligned_start + size.get(),
                end: partition.end,
                used: Rc::new(Cell::new(false)),
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
            .map(|Partition { start, end, used }| if used.get() { 0 } else { end - start })
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
            if partition.used.get() {
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
    used: Rc<Cell<bool>>,
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;

    use super::*;

    #[test]
    fn test_simple() {
        let forty = NonZeroU64::new(40).unwrap();
        let binding_size = forty.get() * u32::MIN_BINDING_SIZE.get();
        let size = binding_size * 4;

        let context = SHARED_CONTEXT.lock().unwrap();
        let mut allocator = GpuAllocator::new(&context, size, "allocation").unwrap();

        let _a = allocator.allocate::<u32>(forty).unwrap();
        let _b = allocator.allocate::<u32>(forty).unwrap();
        let _c = allocator.allocate::<u32>(forty).unwrap();
        let _d = allocator.allocate::<u32>(forty).unwrap();

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
            GpuAllocator::new(&context, u64::MAX, "allocation"),
            Err(GpuAllocatorError::ExceedingMaxBufferSize { .. })
        ));
    }

    #[test]
    fn test_buffer_binding_too_large() {
        let context = SHARED_CONTEXT.lock().unwrap();
        let mut allocator = GpuAllocator::new(&context, 42, "allocation").unwrap();
        assert!(matches!(
            allocator.allocate_raw(
                NonZeroU64::new(u64::MAX).unwrap(),
                NonZeroU64::new(4).unwrap(),
            ),
            Err(GpuAllocatorError::ExceedingMaxBufferBindingSize { .. })
        ));
    }

    #[test]
    fn test_no_space() {
        let forty = NonZeroU64::new(40).unwrap();
        let eighty = NonZeroU64::new(80).unwrap();
        let binding_size = forty.get() * u32::MIN_BINDING_SIZE.get();
        let size = binding_size * 4;

        let context = SHARED_CONTEXT.lock().unwrap();
        let mut allocator = GpuAllocator::new(&context, size, "allocation").unwrap();

        let _a = allocator.allocate::<u32>(forty).unwrap();
        let _b = allocator.allocate::<u32>(forty).unwrap();
        let _c = allocator.allocate::<u32>(forty).unwrap();

        drop(_b);

        assert!(matches!(
            allocator.allocate::<u32>(eighty),
            Err(GpuAllocatorError::FailedToFindSpace { .. }),
        ));
    }
}
