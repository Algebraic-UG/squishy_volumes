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

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct Triangle {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct Opposites {
    pub ab: u32,
    pub bc: u32,
    pub ca: u32,
}

impl<I> From<I> for Triangle
where
    I: Iterator<Item = u32>,
{
    fn from(mut i: I) -> Self {
        Self {
            a: i.next().unwrap(),
            b: i.next().unwrap(),
            c: i.next().unwrap(),
        }
    }
}

impl<I> From<I> for Opposites
where
    I: Iterator<Item = u32>,
{
    fn from(mut i: I) -> Self {
        Self {
            ab: i.next().unwrap(),
            bc: i.next().unwrap(),
            ca: i.next().unwrap(),
        }
    }
}

impl Triangle {
    pub fn iter(&self) -> impl Iterator<Item = &u32> {
        [&self.a, &self.b, &self.c].into_iter()
    }
}

impl Opposites {
    pub fn iter(&self) -> impl Iterator<Item = &u32> {
        [&self.ab, &self.bc, &self.ca].into_iter()
    }
}

impl IntoIterator for Triangle {
    type Item = u32;
    type IntoIter = std::array::IntoIter<u32, 3>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter([self.a, self.b, self.c])
    }
}

impl IntoIterator for Opposites {
    type Item = u32;
    type IntoIter = std::array::IntoIter<u32, 3>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter([self.ab, self.bc, self.ca])
    }
}
