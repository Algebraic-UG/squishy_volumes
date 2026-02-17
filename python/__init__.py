# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import json
from pathlib import Path
import tomllib

import bpy

from .properties import register_properties, unregister_properties
from .progress_update import (
    register_progress_update,
    register_progress_update_toggle,
    unregister_progress_update,
    unregister_progress_update_toggle,
)
from .bridge import Simulation, build_info, test
from .frame_change import register_handler, unregister_handler
from .panels import register_panels, unregister_panels
from .popup import register_popup, unregister_popup
from .view_utils import register_view_utils, unregister_view_utils
from .script_utils import register_script_utils, unregister_script_utils


bl_info = {
    "name": "Squishy Volumes",
    "blender": (4, 2, 0),
    "category": "Physics",
}


@bpy.app.handlers.persistent
def toggle_register(*_):
    unregister()
    register()


def register_blend_file_change_handler():
    if toggle_register not in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.append(toggle_register)
        print("Squishy Volumes load_post registered.")


def unregister_blend_file_change_handler():
    if toggle_register in bpy.app.handlers.load_post:
        bpy.app.handlers.load_post.remove(toggle_register)
        print("Squishy Volumes load_post unregistered.")


class OBJECT_OT_Test(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_test"
    bl_label = "Test"
    bl_options = {"REGISTER", "UNDO"}

    spacing: bpy.props.FloatProperty()  # type: ignore
    layers: bpy.props.IntProperty()  # type: ignore

    def execute(self, context):
        test(self.spacing, float(self.layers))
        return {"FINISHED"}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)


def menu_func(self, _context):
    self.layout.operator(
        OBJECT_OT_Test.bl_idname,
        icon="MODIFIER",
    )


def register():
    bpy.utils.register_class(OBJECT_OT_Test)
    bpy.types.VIEW3D_MT_object.append(menu_func)
    version_rust = build_info()["wrapper"]["crate_info"]["version"]
    manifest_path = Path(__file__).parent / "blender_manifest.toml"
    with manifest_path.open("rb") as f:
        blender_manifest = tomllib.load(f)
    version_python = blender_manifest["version"]
    if version_rust != version_python:
        raise RuntimeError(
            f"Version mismatch! Expected {version_python} but loaded {version_rust}"
        )
    print(f"Squishy Volumes detailed build info: {json.dumps(build_info(), indent=4)}")

    register_popup()
    register_blend_file_change_handler()
    register_properties()
    register_panels()
    register_handler()
    register_progress_update()
    register_progress_update_toggle()
    register_view_utils()
    register_script_utils()


def unregister():
    unregister_script_utils()
    unregister_view_utils()
    unregister_progress_update_toggle()
    unregister_progress_update()
    unregister_handler()
    unregister_panels()
    unregister_properties()
    unregister_blend_file_change_handler()
    unregister_popup()
    Simulation.drop_all()
    bpy.types.VIEW3D_MT_object.remove(menu_func)
    bpy.utils.unregister_class(OBJECT_OT_Test)


if __name__ == "__main__":
    register()
