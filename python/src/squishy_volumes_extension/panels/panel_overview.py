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

import datetime
import json

import bpy

from .. import api
from ..bridge import SimulationHandle, build_info
from ..cache_paths import check_cache_directory, simulation_input_exists
from ..example import EXAMPLE_BENCHMARK, EXAMPLE_BOING_BLOCK, setup_example_simulation
from ..get_preferences import (
    first_run_setup_pending,
    get_default_cache_location,
    get_preferences,
)
from ..operator_base import ReportingOperator, SimulationOperator
from ..popup import popup, show_message
from ..registration import class_registration
from ..squishy_volumes_properties import (
    frame_to_load,
    get_simulation_objects,
    unloaded_simulations,
)
from ..util import force_ui_redraw, read_stats, u64_to_giga_f32


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
            self.warn("", f"The example setup is incomplete:\n{error}")
            return {"FINISHED"}
        select_simulation(context, sim_obj)
        try:
            api.record_input(sim_obj, scene=context.scene)
        except RuntimeError as error:
            self.warn(
                sim_obj.squishy_volumes.uuid,
                f"{sim_obj.name} is set up but not recorded:\n{error}",
            )
            return {"FINISHED"}
        try:
            api.bake(sim_obj)
        except RuntimeError as error:
            self.warn(
                sim_obj.squishy_volumes.uuid,
                f"{sim_obj.name} is set up but not baking:\n{error}",
            )
        return {"FINISHED"}

    def warn(self, uuid: str, message: str):
        show_message(uuid, message)
        self.report({"WARNING"}, message.splitlines()[0])

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=400)


def disk_usage_factor(used_giga_bytes: float, max_giga_bytes: float) -> float:
    if max_giga_bytes <= 0.0:
        return 1.0
    return used_giga_bytes / max_giga_bytes


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
        reloaded = api.reload_all()
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


def draw_first_run_setup(layout: bpy.types.UILayout, context):
    preferences = get_preferences()
    box = layout.box()
    box.label(text="First Run Setup Defaults", icon="PREFERENCES")
    box.prop(preferences, "default_compute_device", text="Device")
    box.prop(preferences, "default_cache_location", text="Cache")
    if context.preferences.use_preferences_save:
        box.operator(SCENE_OT_Squishy_Volumes_Finish_First_Run_Setup.bl_idname)
    else:
        box.label(text="Auto-save preferences is off.", icon="INFO")
        box.operator(
            SCENE_OT_Squishy_Volumes_Finish_First_Run_Setup.bl_idname,
            text="Save Preferences & Done",
        )


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


class SCENE_PT_Squishy_Volumes_Overview(bpy.types.Panel):
    bl_label = f"Overview  -  v{build_info()['wrapper']['crate_info']['version']}"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"

    # No poll: this panel stays visible in every mode so that the sidebar tab
    # keeps its position. Out of Object Mode it only shows the hint below.

    def draw_header_preset(self, context):
        self.layout.operator(
            SCENE_OT_Squishy_Volumes_Copy_Build_Info.bl_idname,
            text="",
            icon="INFO",
            emboss=False,
        )

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        layout = self.layout

        if first_run_setup_pending(get_preferences()):
            draw_first_run_setup(layout, context)
            return

        if context.mode != "OBJECT":
            layout.label(text="Squishy Volumes needs Object Mode", icon="INFO")
            layout.operator(
                "object.mode_set", text="Switch to Object Mode", icon="OBJECT_DATAMODE"
            ).mode = "OBJECT"
            return

        if unloaded_simulations(context):
            layout.operator(SCENE_OT_Squishy_Volumes_Reload_All.bl_idname)
        add_row = layout.row()
        add_row.operator(SCENE_OT_Squishy_Volumes_Add_Simulation.bl_idname, icon="ADD")
        add_row.operator(
            SCENE_OT_Squishy_Volumes_Add_Example_Simulation.bl_idname, icon="ADD"
        )

        if not get_simulation_objects():
            return

        layout.separator()

        for sim_obj in get_simulation_objects():
            sim_props = sim_obj.squishy_volumes
            (header, body) = layout.panel(
                sim_props.uuid,
                default_closed=not simulation_input_exists(sim_props.directory),
            )
            sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
            if sim_handle is not None and sim_handle.last_error is not None:
                col = header.column()
                col.alert = True
                col.label(text=f"{sim_obj.name}: Message")
                header.operator(
                    SCENE_OT_Squishy_Volumes_Show_Message.bl_idname
                ).uuid = sim_props.uuid
            else:
                progress_text = f"{sim_obj.name}: "
                factor = 0.0
                if sim_handle is not None:
                    if (
                        sim_handle.poll_info is not None
                        and sim_handle.poll_info["progress"]
                    ):
                        progress = sim_handle.poll_info["progress"][0]
                        progress_text += progress["label"]
                        completed_steps = progress["completed_steps"]
                        steps_to_completion = progress["steps_to_completion"]
                        progress_text += f" {completed_steps}/{steps_to_completion}"
                        factor = completed_steps / steps_to_completion
                    else:
                        computed = sim_handle.available_frames()
                        if computed == sim_props.bake_frames:
                            progress_text += "Completed: "
                        else:
                            progress_text += "Paused at: "
                        progress_text += f"{computed}/{sim_props.bake_frames}"
                        factor = computed / sim_props.bake_frames
                else:
                    if simulation_input_exists(sim_props.directory):
                        progress_text += "Cache Unloaded"
                    else:
                        progress_text += "Uninitialized"
                header.progress(text=progress_text, factor=factor)

            if body is not None:
                body.prop(sim_obj, "name")
                body.prop(sim_props, "directory")

                col = body.column()
                col.enabled = False
                col.prop(sim_props, "uuid")

                if sim_handle is not None and sim_handle.poll_info is not None:
                    current = u64_to_giga_f32(
                        sim_handle.poll_info["current_bytes_on_disk"]
                    )
                    body.progress(
                        text="Used Disk Space",
                        factor=disk_usage_factor(
                            current, sim_props.max_giga_bytes_on_disk
                        ),
                    )
                body.prop(sim_props, "max_giga_bytes_on_disk")

                row = body.row()
                if sim_handle is None and simulation_input_exists(sim_props.directory):
                    row.operator(
                        SCENE_OT_Squishy_Volumes_Reload.bl_idname,
                        icon="FILE_CACHE",
                    ).uuid = sim_props.uuid
                row.operator(
                    SCENE_OT_Squishy_Volumes_Clone_Simulation.bl_idname,
                    icon="DUPLICATE",
                ).uuid = sim_props.uuid
                row.operator(
                    SCENE_OT_Squishy_Volumes_Remove_Simulation.bl_idname,
                    icon="TRASH",
                ).uuid = sim_props.uuid

                if sim_handle is None:
                    continue
                loaded_frame = frame_to_load(sim_props, context.scene.frame_current)
                if loaded_frame is None:
                    continue

                stats = read_stats(sim_props.directory).get(loaded_frame)
                input_header = sim_handle.input_header()

                total_particle_count = input_header["total_particles"]
                per_object_count = {
                    name: particles["particle_range"]["end"]
                    - particles["particle_range"]["start"]
                    for name, obj in input_header["objects"].items()
                    if (particles := obj.get("Particles")) is not None
                }
                grid_node_count = None if stats is None else stats["grid_node_count"]
                body.label(text="Loaded State Stats")
                box = body.box()
                grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
                grid.label(text="Total particles")
                grid.label(text=f"{total_particle_count}")
                grid.label(text="Active grid nodes")
                grid.label(text=f"{grid_node_count}")
                for name, count in per_object_count.items():
                    grid.label(text=name)
                    grid.label(text=f"{count}")

                if stats is not None and stats["last_frame_time_sec"] is not None:
                    body.label(text="Compute Stats")
                    box = body.box()
                    remaining_time_sec = stats["remaining_time_sec"]
                    last_frame_time_sec = stats["last_frame_time_sec"]
                    last_frame_substeps = stats["last_frame_substeps"]
                    grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
                    grid.label(text="Approx. remaining time")
                    grid.label(
                        text=str(datetime.timedelta(seconds=round(remaining_time_sec)))
                    )
                    grid.label(text="Last frame time")
                    grid.label(text=f"{last_frame_time_sec:0.2f} sec")
                    grid.label(text="Last frame substeps")
                    grid.label(text=f"{last_frame_substeps}")

        layout.separator()

        if len(get_simulation_objects()) > 1:
            layout.prop(
                context.scene.squishy_volumes,
                "selected_simulation",
                text="Select",
            )


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
    SCENE_PT_Squishy_Volumes_Overview,
]


register_panel_overview, unregister_panel_overview = class_registration(classes)
