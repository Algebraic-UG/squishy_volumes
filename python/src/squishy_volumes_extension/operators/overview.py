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

import json

import bpy

from .. import api
from ..bridge import SimulationHandle, build_info
from ..cache_paths import check_cache_directory
from ..example import (
    EXAMPLE_BENCHMARK,
    EXAMPLE_BOING_BLOCK,
    EXAMPLE_MULTI_MATERIAL,
    setup_example_simulation,
)
from ..get_preferences import (
    first_run_setup_pending,
    get_default_cache_location,
    get_preferences,
)
from ..properties import unloaded_simulations
from ..registration import class_registration
from ..util import force_ui_redraw
from .operator_base import ReportingOperator, SimulationOperator
from .popup import popup


class SCENE_OT_Squishy_Volumes_Add_Example_Simulation(
    ReportingOperator, bpy.types.Operator
):
    bl_idname = "scene.squishy_volumes_add_example_simulation"
    bl_label = "Example Setup"
    bl_description = """Start with a prefabricated Squishy Volumes simulation."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    startup_choice: bpy.props.EnumProperty(
        items=[
            (
                EXAMPLE_BOING_BLOCK,
                EXAMPLE_BOING_BLOCK,
                "Just a simple elastic cube falling.",
            ),
            (
                EXAMPLE_BENCHMARK,
                EXAMPLE_BENCHMARK,
                """This sets up a million particle scene to benchmark your hardware.
The UI will be blocked for a few seconds.""",
            ),
            (
                EXAMPLE_MULTI_MATERIAL,
                EXAMPLE_MULTI_MATERIAL,
                "An elastic cube and a fluid sphere falling onto a plane.",
            ),
        ],
        name="Choose Example Simulation",
        description="Choose one of the example simulations to set up.",
        default=EXAMPLE_BOING_BLOCK,
        options=set(),
    )
    new_scene: bpy.props.BoolProperty(
        name="New Scene",
        description="Add the example to a new scene and switch to it.",
        default=False,
        options=set(),
    )

    def run(self, context):
        check_cache_directory(get_default_cache_location())
        if self.new_scene:
            if context.window is None:
                raise api.SquishyVolumesError("There is no window to switch scenes in")
            context.window.scene = bpy.data.scenes.new(self.startup_choice)
        try:
            sim_obj = setup_example_simulation(context, self.startup_choice)
        except RuntimeError as error:
            return self.finish_with_warning(
                "", f"The example setup is incomplete:\n{error}"
            )
        select_simulation(context, sim_obj)
        try:
            api.record_input(sim_obj, scene=context.scene)
        except RuntimeError as error:
            return self.finish_with_warning(
                sim_obj.squishy_volumes.uuid,
                f"{sim_obj.name} is set up but not recorded:\n{error}",
            )
        try:
            api.bake(sim_obj)
        except RuntimeError as error:
            return self.finish_with_warning(
                sim_obj.squishy_volumes.uuid,
                f"{sim_obj.name} is set up but not baking:\n{error}",
            )
        return {"FINISHED"}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=400)


class SCENE_OT_Squishy_Volumes_Add_Simulation(ReportingOperator, bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_simulation"
    bl_label = "Add Simulation"
    bl_description = """Create a new Squishy Volumes simulation object.

There can be multiple simulations at once, but the physics
are completely separate from each other.

Note that this doesn't create any files yet.
It just creates the Blender object to track the simulation."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    sim_name: bpy.props.StringProperty(default="My Simulation")

    def run(self, context):
        sim_obj = api.add_simulation(self.sim_name, scene=context.scene)
        select_simulation(context, sim_obj)
        self.report({"INFO"}, f"Added {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Clone_Simulation(SimulationOperator, bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_clone_simulation"
    bl_label = "Clone"
    bl_description = "Clone this Simulation."
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    def run(self, context):
        sim_obj = self.sim_obj
        clone = api.clone_simulation(sim_obj, scene=context.scene)
        for input_obj in api.input_objects(clone):
            input_obj.select_set(True)
        select_simulation(context, clone)
        self.report({"INFO"}, f"Cloned {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Reload(SimulationOperator, bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_reload"
    bl_label = "Reload"
    bl_description = "Reloads the cache and locks it for this simulation object."
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    def run(self, context):
        sim_obj = self.sim_obj
        api.reload(sim_obj)
        self.report({"INFO"}, f"Reloaded {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Reload_All(ReportingOperator, bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_reload_all"
    bl_label = "Reload All"
    bl_description = """Reloads all simulation caches.
This is useful when reloading a Blender file with multiple simulations."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    def run(self, context):
        reloaded = []
        for sim_obj in unloaded_simulations(context):
            try:
                api.reload(sim_obj)
            except RuntimeError as error:
                if not reloaded:
                    raise
                return self.finish_with_warning(
                    sim_obj.squishy_volumes.uuid,
                    f"Reloaded {len(reloaded)} simulation(s), "
                    f"but not {sim_obj.name}:\n{error}",
                )
            reloaded.append(sim_obj)
        self.report({"INFO"}, f"Reloaded {len(reloaded)} simulation(s).")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Remove_Simulation(
    SimulationOperator, bpy.types.Operator
):
    bl_idname = "scene.squishy_volumes_remove_simulation"
    bl_label = "Remove"
    bl_description = """Remove the simulation from the scene.

This does not clear the cache. If you want to delete (not overwrite) the cache,
please use your OS's file browser."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    def run(self, context):
        sim_obj = self.sim_obj
        name = sim_obj.name
        api.remove_simulation(sim_obj, scene=context.scene)
        self.report({"INFO"}, f"Removed {name}.")
        return {"FINISHED"}


def select_simulation(context, sim_obj: bpy.types.Object):
    context.scene.squishy_volumes.selected_simulation = sim_obj
    force_ui_redraw()


class SCENE_OT_Squishy_Volumes_Show_Message(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_show_message"
    bl_label = "Show"

    uuid: bpy.props.StringProperty()

    def execute(self, context):
        sim = SimulationHandle.get(uuid=self.uuid)
        assert sim is not None
        popup(self.uuid, sim.last_error)

        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Finish_First_Run_Setup(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_finish_first_run_setup"
    bl_label = "Done"
    bl_description = """Keep these defaults and hide the first run setup.
Both can be changed later in the add-on preferences.

If auto-save preferences is off, this saves the preferences."""

    @classmethod
    def poll(cls, context):
        return first_run_setup_pending(get_preferences())

    def execute(self, context):
        get_preferences().first_run_setup_done = True
        if context.preferences.use_preferences_save:
            context.preferences.is_dirty = True
            return {"FINISHED"}
        try:
            bpy.ops.wm.save_userpref()
        except RuntimeError as error:
            self.report({"WARNING"}, f"Preferences were not saved: {error}")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Copy_Build_Info(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_copy_build_info"
    bl_label = "Copy Build Info"
    bl_description = """Copy the detailed build info of Squishy Volumes to the clipboard.
It is also printed to the console.
This is useful when reporting bugs."""

    def execute(self, context):
        detailed_build_info = json.dumps(build_info(), indent=4)
        print(f"Squishy Volumes detailed build info: {detailed_build_info}")
        context.window_manager.clipboard = detailed_build_info
        self.report({"INFO"}, "Copied Squishy Volumes build info to the clipboard.")
        return {"FINISHED"}


classes = [
    SCENE_OT_Squishy_Volumes_Add_Example_Simulation,
    SCENE_OT_Squishy_Volumes_Add_Simulation,
    SCENE_OT_Squishy_Volumes_Clone_Simulation,
    SCENE_OT_Squishy_Volumes_Reload,
    SCENE_OT_Squishy_Volumes_Reload_All,
    SCENE_OT_Squishy_Volumes_Remove_Simulation,
    SCENE_OT_Squishy_Volumes_Show_Message,
    SCENE_OT_Squishy_Volumes_Finish_First_Run_Setup,
    SCENE_OT_Squishy_Volumes_Copy_Build_Info,
]


register_overview_operators, unregister_overview_operators = class_registration(classes)
