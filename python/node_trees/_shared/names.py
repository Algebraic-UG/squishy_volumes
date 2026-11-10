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

import importlib.util
from pathlib import Path
from types import ModuleType

_EXTENSION_DIR = (
    Path(__file__).resolve().parents[2] / "src" / "squishy_volumes_extension"
)


# Loaded by path: importing the extension package would run its __init__,
# which needs the compiled Rust module.
def _load_constants(stem: str) -> ModuleType:
    spec = importlib.util.spec_from_file_location(
        f"squishy_volumes_{stem}", _EXTENSION_DIR / f"{stem}.py"
    )
    assert spec is not None and spec.loader is not None
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


inputs = _load_constants("input_attributes")
outputs = _load_constants("output_attributes")
