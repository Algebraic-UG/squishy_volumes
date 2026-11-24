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

use std::{fmt::Debug, path::Path};

use crate::{Header, InputConsts, InputHeader, InputObject, InputReader, InputWriter};

impl InputHeader {
    pub fn test_input(objects: impl IntoIterator<Item = (String, InputObject)>) -> Self {
        Self {
            consts: InputConsts::test_input(),
            objects: objects.into_iter().collect(),
        }
    }
}

pub fn write_test_input<P: AsRef<Path> + Debug>(
    path: P,
    header: InputHeader,
    number_of_frames: usize,
) -> InputReader {
    let mut writer = InputWriter::new(&path, Header::new(header)).unwrap();
    for _ in 0..number_of_frames {
        writer.start_frame(&bytemuck::Zeroable::zeroed()).unwrap();
    }
    writer.flush().unwrap();
    InputReader::new(path).unwrap()
}
