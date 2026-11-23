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

import tomllib
from pathlib import Path

import bpy

from . import get_preferences
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
from .drivers import register_drivers, unregister_drivers
from .get_preferences import debug_log
from .goals import register_goals, unregister_goals
from .panels import register_panels, unregister_panels
from .popup import register_popup, unregister_popup
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
from .squishy_volumes_properties import register_properties, unregister_properties
from .view_utils import register_view_utils, unregister_view_utils

bl_info = {
    "name": "Squishy Volumes",
    "category": "Physics",
}


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

    register_popup()
    register_properties()
    register_panels()
    register_goals()
    register_progress_update()
    register_progress_update_toggle()
    register_view_utils()
    register_append_handler()
    register_prune_simulation_handles()
    register_drop_simulation_handles_on_load()
    register_drivers()

    bpy.app.timers.register(reload_assets, first_interval=0)


def unregister():
    unregister_drivers()
    unregister_drop_simulation_handles_on_load()
    unregister_prune_simulation_handles()
    unregister_append_handler()
    unregister_view_utils()
    unregister_progress_update_toggle()
    unregister_progress_update()
    unregister_goals()
    unregister_panels()
    unregister_properties()
    unregister_popup()
    unregister_preferences()
    SimulationHandle.drop_all()


if __name__ == "__main__":
    register()
