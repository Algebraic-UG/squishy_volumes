// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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
    label: &'static str,
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
        DownloadToHostReady {
            label: self.source.label(),
            data_slice,
        }
    }
}

impl DownloadToHostReady<'_> {
    pub fn map<T, U, F>(&self, f: F) -> Result<Vec<U>, GpuError>
    where
        T: bytemuck::Pod,
        F: FnMut(&T) -> U,
    {
        Ok(
            bytemuck::cast_slice::<u8, T>(&self.data_slice.get_mapped_range().map_err(
                |error| GpuError::MapRangeError {
                    label: self.label,
                    error,
                },
            )?)
            .iter()
            .map(f)
            .collect(),
        )
    }

    pub fn to_vec<T: bytemuck::Pod>(&self) -> Result<Vec<T>, GpuError> {
        Ok(
            bytemuck::cast_slice(&self.data_slice.get_mapped_range().map_err(|error| {
                GpuError::MapRangeError {
                    label: self.label,
                    error,
                }
            })?)
            .to_vec(),
        )
    }
}
