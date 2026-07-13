// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

pub fn frame_path(cache_dir: impl AsRef<std::path::Path>, frame: usize) -> std::path::PathBuf {
    cache_dir.as_ref().join(format!("frame_{frame:05}.bin"))
}

pub fn get_frame_number(frame_path: impl AsRef<std::path::Path>) -> Option<usize> {
    if !frame_path.as_ref().is_file() {
        return None;
    }
    let file_name = frame_path.as_ref().file_name()?.to_str()?;
    if !file_name.starts_with("frame_") || !file_name.ends_with(".bin") {
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
