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
import tomllib
from pathlib import Path

import bpy

from . import api, get_preferences
from .append_handler import register_append_handler, unregister_append_handler
from .assets import reload_assets
from .bridge import (
    SimulationHandle,
    build_info,
    build_summary,
    register_drop_simulation_handles_on_load,
    register_prune_simulation_handles,
    unregister_drop_simulation_handles_on_load,
    unregister_prune_simulation_handles,
)
from .get_preferences import debug_log
from .operators import register_operators, unregister_operators
from .panels import register_panels, unregister_panels
from .preferences import (
    register_preferences,
    unregister_preferences,
)
from .progress_update import (
    register_progress_update,
    register_progress_update_toggle,
    unregister_progress_update,
    unregister_progress_update_toggle,
)
from .properties import register_properties, unregister_properties

bl_info = {
    "name": "Squishy Volumes",
    "category": "Physics",
}

SCRIPTING_ALIAS = "squishy_volumes"


def register():
    register_preferences()
    get_preferences.apply_environment_defaults(get_preferences.get_preferences())

    version_rust = build_info()["wrapper"]["crate_info"]["version"]
    manifest_path = Path(__file__).parent / "blender_manifest.toml"
    with manifest_path.open("rb") as f:
        blender_manifest = tomllib.load(f)
    version_python = blender_manifest["version"]
    if version_rust != version_python:
        raise RuntimeError(
            f"Version mismatch! Expected {version_python} but loaded {version_rust}"
        )
    debug_log(build_summary())

    register_properties()
    register_operators()
    register_panels()
    register_progress_update()
    register_progress_update_toggle()
    register_append_handler()
    register_prune_simulation_handles()
    register_drop_simulation_handles_on_load()

    bpy.app.timers.register(reload_assets, first_interval=0)

    sys.modules[SCRIPTING_ALIAS] = api


def unregister():
    if sys.modules.get(SCRIPTING_ALIAS) is api:
        del sys.modules[SCRIPTING_ALIAS]
    unregister_drop_simulation_handles_on_load()
    unregister_prune_simulation_handles()
    unregister_append_handler()
    unregister_progress_update_toggle()
    unregister_progress_update()
    unregister_panels()
    unregister_operators()
    unregister_properties()
    unregister_preferences()
    SimulationHandle.drop_all()


if __name__ == "__main__":
    register()
