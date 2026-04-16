// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

pub struct DownloadsToHost(Vec<DownloadToHost>);

pub struct DownloadsToHostReady<'a>(Vec<DownloadToHostReady<'a>>);

impl DownloadsToHost {
    pub fn new(context: &GpuContext, sources: impl IntoIterator<Item = Allocation>) -> Self {
        Self(
            sources
                .into_iter()
                .map(|source| DownloadToHost::new(context, source))
                .collect(),
        )
    }

    pub fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        self.0.iter().for_each(|download| download.copy(encoder));
    }

    pub fn prep<'a>(&'a self) -> DownloadsToHostReady<'a> {
        DownloadsToHostReady(self.0.iter().map(|download| download.prep()).collect())
    }
}

impl<'a, const N: usize> TryFrom<DownloadsToHostReady<'a>> for [DownloadToHostReady<'a>; N] {
    type Error = <Self as TryFrom<Vec<DownloadToHostReady<'a>>>>::Error;
    fn try_from(value: DownloadsToHostReady<'a>) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

pub struct DownloadToHost {
    source: Allocation,
    target: wgpu::Buffer,
}

#[derive(Debug)]
pub struct DownloadToHostReady<'a> {
    data_slice: wgpu::BufferSlice<'a>,
}

impl DownloadToHost {
    pub fn new(context: &GpuContext, source: Allocation) -> Self {
        let target = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("download_{}", source.label())),
            size: source.size().get(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self { source, target }
    }

    pub fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(
            self.source.buffer(),
            self.source.offset(),
            &self.target,
            0,
            Some(self.source.size().get()),
        );
    }

    pub fn prep<'a>(&'a self) -> DownloadToHostReady<'a> {
        let data_slice = self.target.slice(..);
        data_slice.map_async(wgpu::MapMode::Read, |_| {});
        DownloadToHostReady { data_slice }
    }
}

impl DownloadToHostReady<'_> {
    pub fn to_vec<T: bytemuck::Pod>(&self) -> Vec<T> {
        bytemuck::cast_slice(&self.data_slice.get_mapped_range()).to_vec()
    }
}
