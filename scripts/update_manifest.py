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
cargo_toml_path = repo_root / "rust" / "crates" / "wrap" / "Cargo.toml"
manifest_template_path = scripts_dir / "blender_manifest_template.toml"
extension_dir = repo_root / "python" / "src" / "squishy_volumes_extension"
wheels_dir = extension_dir / "wheels"
manifest_path = extension_dir / "blender_manifest.toml"

with cargo_toml_path.open("r") as f:
    cargo_toml = toml.load(f)
version = cargo_toml["package"]["version"]

wheel_paths = sorted(wheels_dir.glob("*.whl"))
if not wheel_paths:
    print(f"Found no wheels in {wheels_dir}", file=sys.stderr)
    sys.exit(1)

with manifest_template_path.open("r") as f:
    manifest = toml.load(f)

manifest["version"] = version
manifest["wheels"] = [f"./wheels/{wheel.name}" for wheel in wheel_paths]

with manifest_path.open("w") as f:
    toml.dump(manifest, f)

print(f"Updated {manifest_path} to {version} with {len(wheel_paths)} wheel(s).")
