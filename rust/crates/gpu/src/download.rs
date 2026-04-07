// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use super::*;

pub struct DownloadsToHost<'a>(Vec<DownloadToHost<'a>>);

pub struct DownloadsToHostReady<'a>(Vec<DownloadToHostReady<'a>>);

impl<'a> DownloadsToHost<'a> {
    pub fn new(
        context: &GpuContext,
        sources_and_labels: impl IntoIterator<Item = (&'a wgpu::Buffer, &'static str)>,
    ) -> Self {
        Self(
            sources_and_labels
                .into_iter()
                .map(|(source, label)| DownloadToHost::new(context, source, label))
                .collect(),
        )
    }

    pub fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        self.0.iter().for_each(|download| download.copy(encoder));
    }

    pub fn prep(&'a self) -> DownloadsToHostReady<'a> {
        DownloadsToHostReady(self.0.iter().map(|download| download.prep()).collect())
    }
}

impl<'a, const N: usize> TryFrom<DownloadsToHostReady<'a>> for [DownloadToHostReady<'a>; N] {
    type Error = <Self as TryFrom<Vec<DownloadToHostReady<'a>>>>::Error;
    fn try_from(value: DownloadsToHostReady<'a>) -> Result<Self, Self::Error> {
        value.0.try_into()
    }
}

pub struct DownloadToHost<'a> {
    source: &'a wgpu::Buffer,
    target: wgpu::Buffer,
}

#[derive(Debug)]
pub struct DownloadToHostReady<'a> {
    data_slice: wgpu::BufferSlice<'a>,
}

impl<'a> DownloadToHost<'a> {
    pub fn new(context: &GpuContext, source: &'a wgpu::Buffer, label: &'static str) -> Self {
        let target = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some(&format!("download_{label}")),
            size: source.size(),
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        Self { source, target }
    }

    pub fn copy(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(self.source, 0, &self.target, 0, None);
    }

    pub fn prep(&'a self) -> DownloadToHostReady<'a> {
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
