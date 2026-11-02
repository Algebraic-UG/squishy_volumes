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

fn main() {
    let compiler = wesl::Wesl::new("src/shaders");

    for shader_path in std::fs::read_dir("src/shaders")
        .unwrap()
        .filter_map(|res| res.ok())
        .map(|dir_entry| dir_entry.path())
        .filter_map(|path| {
            path.extension()
                .is_some_and(|ext| ext == "wesl")
                .then_some(path)
        })
    {
        println!("Found shader: {shader_path:?}");
        let name = shader_path.file_stem().unwrap().to_str().unwrap();
        compiler.build_artifact(&format!("package::{name}",).parse().unwrap(), name);
    }
}
