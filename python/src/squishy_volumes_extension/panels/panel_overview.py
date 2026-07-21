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

import re

import bpy

import datetime

import json
import os
import uuid
from pathlib import Path


from ..bridge import SimulationHandle
from ..frame_change import sync_simulation
from ..popup import popup
from ..progress_update import cleanup_markers
from ..squishy_volumes_properties import (
    update_directory,
    add_fields_from,
    get_input_objects,
    get_simulation_object_with_uuid,
    get_input_objects_with_uuid,
    get_output_objects_with_uuid,
    get_output_objects,
    get_simulation_objects,
    TYPE_SIMULATION,
    TYPE_NONE,
    locked_simulations,
    unloaded_simulations,
)
from ..util import (
    force_ui_redraw,
    simulation_input_exists,
    simulation_locked,
)
from ..example import EXAMPLE_BOING_BLOCK, EXAMPLE_BENCHMARK, setup_example_simulation


class SCENE_OT_Squishy_Volumes_Add_Example_Simulation(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_example_simulation"
    bl_label = "Example Setup"
    bl_description = """Start with a prefabricated Squishy Volumes simulation."""
    bl_options = {"REGISTER", "UNDO"}

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
        name="Chose Example Simulation",
        description="Chose one of the example simulations to set up.",
        default=EXAMPLE_BOING_BLOCK,
        options=set(),
    )  # type: ignore

    def execute(self, context):
        setup_example_simulation(context, self.startup_choice)
        return {"FINISHED"}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)


class SCENE_OT_Squishy_Volumes_Add_Simulation(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_simulation"
    bl_label = "Add Simulation"
    bl_description = """Create a new Squishy Volumes simulation object.

There can be multiple simulations at once, but the physics
are completely separate from each other.

Note that this doesn't create any files yet.
It just creates the Blender object to track the simulation."""
    bl_options = {"REGISTER", "UNDO"}

    name: bpy.props.StringProperty()  # type: ignore
    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        sim_obj = bpy.data.objects.new(name=self.name, object_data=None)
        sim_obj.empty_display_type = "CUBE"
        sim_obj.lock_location = (True,) * 3
        sim_obj.lock_rotation = (True,) * 3
        sim_obj.lock_scale = (True,) * 3

        # It won't persist otherwise.
        # https://github.com/Algebraic-UG/squishy_volumes/issues/247
        sim_obj.use_fake_user = True

        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

        sim_props.type = TYPE_SIMULATION
        sim_props.uuid = self.uuid

        update_directory(sim_props, context)

        force_ui_redraw()

        self.report({"INFO"}, f"Added {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Reload(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_reload"
    bl_label = "Reload"
    bl_description = "Reloads the cache and locks it for this simulation object."
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)

        sim_obj.squishy_volumes.has_loaded_frame = False  # ty:ignore[unresolved-attribute]

        sim_handle = SimulationHandle.load(
            uuid=self.uuid,
            directory=sim_obj.squishy_volumes.directory,  # ty:ignore[unresolved-attribute]
        )

        sync_simulation(
            sim_obj.squishy_volumes,  # ty:ignore[unresolved-attribute]
            sim_handle,
            context.scene.frame_current,
        )

        self.report({"INFO"}, f"Reloaded {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Reload_All(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_reload_all"
    bl_label = "Reload All"
    bl_description = """Reloads all simulation caches.
This is useful when reloading a Blender file with multiple simulations."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        for sim_obj in unloaded_simulations(context):
            sim_props = sim_obj.squishy_volumes
            lock_file = Path(sim_props.directory) / "lock"
            if os.path.exists(lock_file):
                os.remove(lock_file)
                self.report({"INFO"}, "Removed lock file.")
            sim_props.has_loaded_frame = False

            sim_handle = SimulationHandle.load(
                uuid=sim_props.uuid, directory=sim_props.directory
            )

            sync_simulation(sim_props, sim_handle, context.scene.frame_current)
            self.report({"INFO"}, "Reloaded simulation.")

        return {"FINISHED"}

    def invoke(self, context, event):
        if locked_simulations():
            return context.window_manager.invoke_props_dialog(self)
        else:
            return self.execute(context)

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        locked = locked_simulations()
        if locked:
            self.layout.label(text="WARNING: these caches contain lock files:")
            for sim_obj in locked:
                self.layout.label(text=f"{sim_obj.name}")
            self.layout.label(text="Confirm to remove them.")


class SCENE_OT_Squishy_Volumes_Remove_Simulation(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_remove_simulation"
    bl_label = "Remove"
    bl_description = """Remove the simulation from the scene.

This does not clear the cache. If you want to delete (not overwrite) the cache,
please use your OS's file browser."""
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)

        for obj in get_input_objects_with_uuid(self.uuid):
            obj.squishy_volumes.uuid = "unassigned"  # ty:ignore[unresolved-attribute]
            obj.squishy_volumes.type = TYPE_NONE  # ty:ignore[unresolved-attribute]
        for obj in get_output_objects_with_uuid(self.uuid):
            obj.squishy_volumes.uuid = "unassigned"  # ty:ignore[unresolved-attribute]
            obj.squishy_volumes.type = TYPE_NONE  # ty:ignore[unresolved-attribute]

        cleanup_markers(sim_obj)

        bpy.data.objects.remove(sim_obj)

        self.report({"INFO"}, "Removed simulation")

        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Remove_Lock_File(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_remove_lock_file"
    bl_label = "Remove Lock"
    bl_description = """Use with care!

If the lock file is present, it usually means that another simulation is using this cache.
However, the lock file can remain after a crash, in which case it must be deleted."""
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        lock_file = Path(sim_obj.squishy_volumes.directory) / "lock"  # ty:ignore[unresolved-attribute]
        if os.path.exists(lock_file):
            os.remove(lock_file)
            self.report({"INFO"}, f"Removed lock file for {sim_obj.name} .")
        else:
            self.report({"WARNING"}, f"No lock file present for {sim_obj.name}?")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Show_Message(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_show_message"
    bl_label = "Show"

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        popup(self.uuid)

        return {"FINISHED"}


class SCENE_PT_Squishy_Volumes_Overview(bpy.types.Panel):
    bl_label = "Overview"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"

    @classmethod
    def poll(cls, context):
        return context.mode == "OBJECT"

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        layout = self.layout

        if unloaded_simulations(context):
            layout.operator(SCENE_OT_Squishy_Volumes_Reload_All.bl_idname)
        add_row = layout.row()
        add_op = add_row.operator(
            SCENE_OT_Squishy_Volumes_Add_Simulation.bl_idname, icon="ADD"
        )
        add_op.name = "My Simulation"
        add_op.uuid = str(uuid.uuid4())
        add_row.operator(
            SCENE_OT_Squishy_Volumes_Add_Example_Simulation.bl_idname, icon="ADD"
        )

        if not get_simulation_objects():
            return

        layout.separator()

        for sim_obj in get_simulation_objects():
            sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
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
                    if sim_handle.progress is not None and sim_handle.progress:
                        progress = sim_handle.progress[0]
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
                    if simulation_locked(sim_props.directory):
                        progress_text += "Cache Locked!"
                    elif simulation_input_exists(sim_props.directory):
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

                col = body.column()
                col.prop(sim_props, "sync")
                col.prop(sim_props, "max_giga_bytes_on_disk")

                row = body.row()
                if sim_handle is None and simulation_locked(sim_props.directory):
                    row.operator(
                        SCENE_OT_Squishy_Volumes_Remove_Lock_File.bl_idname,
                        icon="WARNING_LARGE",
                    ).uuid = sim_props.uuid
                elif sim_handle is None and simulation_input_exists(
                    sim_props.directory
                ):
                    row.operator(
                        SCENE_OT_Squishy_Volumes_Reload.bl_idname,
                        icon="FILE_CACHE",
                    ).uuid = sim_props.uuid
                row.operator(
                    SCENE_OT_Squishy_Volumes_Remove_Simulation.bl_idname,
                    icon="TRASH",
                ).uuid = sim_props.uuid

                if sim_handle is None:
                    continue
                stats = sim_handle.stats()
                state = stats["state"]
                compute = stats["compute"]
                bytes_on_disk = stats["bytes_on_disk"]

                body.label(text="Misc. Stats")
                box = body.box()
                grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
                grid.label(text="Currently used")
                grid.label(text=f"{bytes_on_disk * 1e-9:.2f} GB")

                total_particle_count = state["total_particle_count"]
                grid_node_count = state["grid_node_count"]
                per_object_count = state["per_object_count"]
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

                if compute is not None:
                    body.label(text="Compute Stats")
                    box = body.box()
                    remaining_time_sec = compute["remaining_time_sec"]
                    last_frame_time_sec = compute["last_frame_time_sec"]
                    last_frame_substeps = compute["last_frame_substeps"]
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
    SCENE_OT_Squishy_Volumes_Reload,
    SCENE_OT_Squishy_Volumes_Reload_All,
    SCENE_OT_Squishy_Volumes_Remove_Simulation,
    SCENE_OT_Squishy_Volumes_Remove_Lock_File,
    SCENE_OT_Squishy_Volumes_Show_Message,
    SCENE_PT_Squishy_Volumes_Overview,
]


def register_panel_overview():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_overview():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
