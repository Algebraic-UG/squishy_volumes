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

import bpy

from .. import api
from ..properties import (
    get_selected_simulation_object,
    get_simulation_objects,
)
from ..registration import class_registration
from .operator_base import ReportingOperator

_simulation_items: list[tuple[str, str, str]] = []
_camera_items: list[tuple[str, str, str]] = []
NONE = "NONE"


def simulation_items(_operator, _context) -> list[tuple[str, str, str]]:
    _simulation_items[:] = [
        (sim_obj.name, sim_obj.name, "") for sim_obj in get_simulation_objects()
    ] or [(NONE, "No simulation", "")]
    return _simulation_items


def camera_items(_operator, context) -> list[tuple[str, str, str]]:
    _camera_items[:] = [
        (obj.name, obj.name, "")
        for obj in context.scene.objects
        if obj.type == "CAMERA"
    ] or [(NONE, "No camera in the scene", "")]
    return _camera_items


class SCENE_OT_Squishy_Volumes_Set_Up_Render(ReportingOperator, bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_set_up_render"
    bl_label = "Squishy Volumes Set Up Render"
    bl_description = """Apply a render preset for a simulation to the current scene.

This sets the scene camera, frame range, resolution, engine and video output.
Render the animation afterwards to get the video."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    simulation: bpy.props.EnumProperty(
        items=simulation_items,
        name="Simulation",
        description="The simulation whose baked frames are rendered.",
        options=set(),
    )
    preset: bpy.props.EnumProperty(
        items=[
            (api.PREVIEW, "Preview", "A small, quick video to check the simulation."),
            (api.SHOWCASE, "Showcase", "A larger, nicer video to show the simulation."),
        ],
        name="Preset",
        default=api.PREVIEW,
        options=set(),
    )
    camera: bpy.props.EnumProperty(
        items=camera_items,
        name="Camera",
        description="The camera to render with, it becomes the scene camera.",
        options=set(),
    )

    def invoke(self, context, event):
        sim_obj = get_selected_simulation_object(context.scene)
        if sim_obj is not None:
            self.simulation = sim_obj.name
        if context.scene.camera is not None:
            self.camera = context.scene.camera.name
        return context.window_manager.invoke_props_dialog(self)

    def run(self, context):
        sim_obj = api.find_simulation_by_name(self.simulation)
        camera = context.scene.objects.get(self.camera)
        api.set_up_render(sim_obj, self.preset, camera=camera, scene=context.scene)
        self.report(
            {"INFO"},
            f"Set up the {self.preset.lower()} render of {sim_obj.name}.",
        )
        return {"FINISHED"}


def menu_func_set_up_render(self, _context):
    self.layout.separator()
    self.layout.operator(
        SCENE_OT_Squishy_Volumes_Set_Up_Render.bl_idname, icon="RENDER_ANIMATION"
    )


register_render_setup, unregister_render_setup = class_registration(
    [SCENE_OT_Squishy_Volumes_Set_Up_Render],
    render_menu_entries=[menu_func_set_up_render],
)
