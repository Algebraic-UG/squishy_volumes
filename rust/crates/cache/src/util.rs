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

pub fn frame_path(cache_dir: impl AsRef<std::path::Path>, frame: usize) -> std::path::PathBuf {
    cache_dir.as_ref().join(format!("frame_{frame:05}.abc"))
}

// the python side reads this file, keep the name in sync
pub fn stats_path(cache_dir: impl AsRef<std::path::Path>) -> std::path::PathBuf {
    cache_dir.as_ref().join("stats.csv")
}

pub fn get_frame_number(frame_path: impl AsRef<std::path::Path>) -> Option<usize> {
    if !frame_path.as_ref().is_file() {
        return None;
    }
    let file_name = frame_path.as_ref().file_name()?.to_str()?;
    if !file_name.starts_with("frame_") || !file_name.ends_with(".abc") {
        return None;
    }
    file_name
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse::<usize>()
        .ok()
}
