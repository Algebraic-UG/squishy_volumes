# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Blended MPM extension.
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
import bpy

from ..properties.util import get_selected_simulation
from ..bridge import (
    computing,
    available_frames,
    context_exists,
    pause_compute,
    start_compute,
    start_compute_initial_frame,
)


class OBJECT_OT_Blended_MPM_Bake_Initial_Frame(bpy.types.Operator):
    bl_idname = "object.blended_mpm_bake_initial_frame"
    bl_label = "Create Simulation Sate"
    bl_description = """Create the initial simulation state from the input (Frame #0).

If the resolution is high, it might take some time.
It can be canceled.
Outputs become available as the initial state is created."""
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context)
        return (
            simulation is not None
            and context_exists(simulation)
            and not computing(simulation)
            and available_frames(simulation) == 0
        )

    def execute(self, context):
        simulation = get_selected_simulation(context)
        simulation.last_exception = ""
        start_compute_initial_frame(simulation)
        self.report({"INFO"}, f"Creating first frame of {simulation.name}.")
        return {"FINISHED"}


class OBJECT_OT_Blended_MPM_Bake_Start_From_Latest(bpy.types.Operator):
    bl_idname = "object.blended_mpm_bake_start_from_latest"
    bl_label = "Bake (from latest)"
    bl_description = """Continue baking physics.

This uses the latest state available and runs the simulation
either until the desired number of frames is reached
or cancellation occurs due to user input or error."""
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context)
        return (
            simulation is not None
            and context_exists(simulation)
            and not computing(simulation)
            and available_frames(simulation) < simulation.bake_frames
        )

    def execute(self, context):
        simulation = get_selected_simulation(context)
        simulation.last_exception = ""
        start_compute(simulation, available_frames(simulation))
        self.report({"INFO"}, f"Commence baking of {simulation.name}.")
        return {"FINISHED"}


class OBJECT_OT_Blended_MPM_Bake_Start_From_Loaded(bpy.types.Operator):
    bl_idname = "object.blended_mpm_bake_start_from_loaded"
    bl_label = "Bake"
    bl_description = """Restart baking physics.

This uses the displayed state and runs the simulation
either until the desired number of frames is reached
or cancellation occurs due to user input or error.

Note that this discards already computed frames that
come after the displayed one."""
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context)
        return (
            simulation is not None
            and context_exists(simulation)
            and not computing(simulation)
            and simulation.loaded_frame < simulation.bake_frames - 1
        )

    def execute(self, context):
        simulation = get_selected_simulation(context)
        start_compute(simulation, simulation.loaded_frame + 1)
        simulation.last_exception = ""
        self.report({"INFO"}, f"Commence baking of {simulation.name}.")
        return {"FINISHED"}


class OBJECT_OT_Blended_MPM_Bake_Pause(bpy.types.Operator):
    bl_idname = "object.blended_mpm_bake_pause"
    bl_label = "Pause"
    bl_description = "Pause the computation of the simulation frames."
    bl_options = {"REGISTER"}

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context)
        return (
            simulation is not None
            and context_exists(simulation)
            and computing(simulation)
        )

    def execute(self, context):
        simulation = get_selected_simulation(context)
        pause_compute(simulation)
        self.report({"INFO"}, f"Baking of {simulation.name} paused.")
        return {"FINISHED"}


def recursive_progress(layout, progress):
    name = progress["name"]
    completed_steps = progress["completed_steps"]
    steps_to_completion = progress["steps_to_completion"]
    layout.progress(
        text=f"{name}: {completed_steps}/{steps_to_completion}",
        factor=completed_steps / steps_to_completion,
    )
    for sub in progress["sub_tasks"]:
        recursive_progress(layout, sub)


class OBJECT_PT_Blended_MPM_Bake(bpy.types.Panel):
    bl_label = "Bake"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Blended MPM"
    bl_options = set()

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context)
        return (
            context.mode == "OBJECT"
            and simulation is not None
            and context_exists(simulation)
        )

    def draw(self, context):
        simulation = get_selected_simulation(context)
        col = self.layout.column()
        col.enabled = not computing(simulation)
        if available_frames(simulation) == 0:
            tut = col.column()
            tut.alert = context.scene.blended_mpm_scene.tutorial_active
            tut.operator("object.blended_mpm_bake_initial_frame", icon="PHYSICS")
        else:
            col.prop(simulation, "time_step")
            # TODO: make implicit viable
            # col.prop(simulation, "explicit")
            # col.prop(simulation, "debug_mode")
            col.prop(simulation, "bake_frames")

            row = self.layout.row()
            row.operator("object.blended_mpm_bake_start_from_latest", icon="PHYSICS")
            if (
                simulation.loaded_frame != -1
                and simulation.loaded_frame + 1 != available_frames(simulation)
            ):
                row.operator(
                    "object.blended_mpm_bake_start_from_loaded",
                    text=f"Rebake from #{simulation.loaded_frame}",
                    icon="PHYSICS",
                )
        self.layout.operator("object.blended_mpm_bake_pause", icon="CANCEL")

        if simulation.progress_json_string:
            recursive_progress(self.layout, json.loads(simulation.progress_json_string))


classes = [
    OBJECT_OT_Blended_MPM_Bake_Initial_Frame,
    OBJECT_OT_Blended_MPM_Bake_Start_From_Latest,
    OBJECT_OT_Blended_MPM_Bake_Start_From_Loaded,
    OBJECT_OT_Blended_MPM_Bake_Pause,
    OBJECT_PT_Blended_MPM_Bake,
]


def register_panel_bake():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_bake():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
