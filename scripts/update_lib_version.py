# SPDX-License-Identifier: AGPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import sys
from pathlib import Path

import toml

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
wrap_dir = repo_root / "rust" / "crates" / "wrap"
cargo_toml_path = wrap_dir / "Cargo.toml"
python_shim_path = (
    repo_root / "python" / "src" / "squishy_volumes_extension" / "shim.py"
)
rust_shim_path = wrap_dir / "src" / "shim.rs"

with cargo_toml_path.open("r") as f:
    cargo_toml = toml.load(f)
version = cargo_toml["package"]["version"]
version_and_addendum = version.split("-")
major, minor, patch = version_and_addendum[0].split(".")

name = "squishy_volumes_wrap"
versioned_name = f"{name}_{major}_{minor}_{patch}"

if len(version_and_addendum) > 1:
    versioned_name = f"{versioned_name}_{version_and_addendum[1]}"

with python_shim_path.open("w") as f:
    f.write(
        f"""# This file is generated to alleviate https://github.com/Algebraic-UG/squishy_volumes/issues/83

import {versioned_name} as {name}  # noqa: F401  # ty: ignore[unresolved-import]
"""
    )

with rust_shim_path.open("w") as f:
    f.write(
        f"""// This file is generated to alleviate https://github.com/Algebraic-UG/squishy_volumes/issues/83

use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
pub fn {versioned_name}(m: &Bound<'_, PyModule>) -> PyResult<()> {{
    super::{name}(m)
}}
"""
    )

print(f"Updated {python_shim_path} and {rust_shim_path} to {version}.")
