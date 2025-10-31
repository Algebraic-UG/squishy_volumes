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
import os
import uuid
from pathlib import Path

import bpy

from ..bridge import (
    available_frames,
    computing,
    context_exists,
    drop_context,
    load_simulation,
    stats,
)
from ..frame_change import sync_simulation
from ..nodes.drivers import update_drivers
from ..popup import popup
from ..progress_update import cleanup_markers
from ..properties.squishy_volumes_simulation import Squishy_Volumes_Simulation_Props
from ..properties.util import (
    get_input_objects,
    get_output_objects,
    get_selected_simulation,
)
from ..util import (
    force_ui_redraw,
    get_simulation_by_uuid,
    get_simulation_idx_by_uuid,
    simulation_cache_exists,
    simulation_cache_locked,
    tutorial_msg,
    locked_simulations,
    unloaded_simulations,
)


class SCENE_OT_Squishy_Volumes_Add_Simulation(
    bpy.types.Operator, Squishy_Volumes_Simulation_Props
):
    bl_idname = "scene.squishy_volumes_add_simulation"
    bl_label = "Add Simulation"
    bl_description = """Create a new Squishy Volumes simulation.

There can be multiple simulations at once
and they can share input geometries, but the physics
are completely separate from each other."""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return (
            not context.scene.squishy_volumes_scene.tutorial_active
            or not context.scene.squishy_volumes_scene.simulations
        )

    def execute(self, context):
        simulations = context.scene.squishy_volumes_scene.simulations

        new_simulation = simulations.add()
        new_simulation.uuid = str(uuid.uuid4())
        new_simulation.name = self.name
        new_simulation.cache_directory = self.cache_directory
        new_simulation.max_giga_bytes_on_disk = self.max_giga_bytes_on_disk

        force_ui_redraw()
        return {"FINISHED"}

    def invoke(self, context, _):
        if not context.scene.squishy_volumes_scene.tutorial_active:
            return self.execute(context)
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        self.layout.prop(self, "name")
        self.layout.prop(self, "cache_directory")
        self.layout.prop(self, "max_giga_bytes_on_disk")
        tutorial_msg(
            self.layout,
            context,
            """\
            You're about to add a new simulation.

            That means you are creating a *cache* directory
            where all the inputs and outputs of
            your simulation are stored!

            You can leave everything as default for now
            and press OK.""",
        )


class SCENE_OT_Squishy_Volumes_Reload(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_reload"
    bl_label = "Reload"
    bl_description = "Reloads the cache"
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        simulation = get_simulation_by_uuid(self.uuid)
        simulation.last_exception = ""
        simulation.loaded_frame = -1
        load_simulation(simulation)
        sync_simulation(simulation, context.scene.frame_current)
        self.report({"INFO"}, "Reloaded simulation.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Reload_All(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_reload_all"
    bl_label = "Reload All"
    bl_description = """Reloads all simulation caches.
This is useful when reloading a Blender filer with multiple simulations."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        for simulation in unloaded_simulations(context):
            lock_file = Path(simulation.cache_directory) / "lock"
            if os.path.exists(lock_file):
                os.remove(lock_file)
                self.report({"INFO"}, "Removed lock file.")

            simulation.last_exception = ""
            simulation.loaded_frame = -1
            load_simulation(simulation)
            sync_simulation(simulation, context.scene.frame_current)
            self.report({"INFO"}, "Reloaded simulation.")

        return {"FINISHED"}

    def invoke(self, context, _event):
        if locked_simulations(context):
            return context.window_manager.invoke_props_dialog(self)
        else:
            return self.execute(context)

    def draw(self, context):
        locked = locked_simulations(context)
        if locked:
            self.layout.label(text="WARNING: these caches contain lock files:")
            for simulation in locked:
                self.layout.label(text=f"{simulation.name}")
            self.layout.label(text="Confirm to remove them.")


class SCENE_OT_Squishy_Volumes_Remove_Simulation(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_remove_simulation"
    bl_label = "Remove"
    bl_description = """Remove the simulation from the scene.

This does not clear the cache. If you want to delete (not overwrite) the cache,
please use your OS's file browser."""
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        simulation = get_simulation_by_uuid(self.uuid)
        idx = get_simulation_idx_by_uuid(self.uuid)
        selected_uuid = get_selected_simulation(context).uuid

        for obj in get_input_objects(simulation):
            all_settings = obj.squishy_volumes_object.simulation_specific_settings
            all_settings.remove(
                next(
                    i
                    for i, settings in enumerate(all_settings)
                    if settings.simulation_uuid == simulation.uuid
                )
            )
        for obj in get_output_objects(simulation):
            obj.squishy_volumes_object.simulation_uuid = ""

        update_drivers(idx)
        cleanup_markers(simulation)
        drop_context(simulation)

        simulations = context.scene.squishy_volumes_scene.simulations

        # Note:
        # This actually invalidates the element!
        # It's UB to continue using simulation
        simulations.remove(idx)

        if simulations and self.uuid == selected_uuid:
            context.scene.squishy_volumes_scene.selected_simulation = simulations[
                0
            ].uuid
            self.report({"INFO"}, "Updated simulation selection.")

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

    def execute(self, _context):
        simulation = get_simulation_by_uuid(self.uuid)
        lock_file = Path(simulation.cache_directory) / "lock"
        if os.path.exists(lock_file):
            os.remove(lock_file)
            self.report({"INFO"}, "Removed lock file.")
        else:
            self.report({"INFO"}, "No lock file present.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Show_Message(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_show_message"
    bl_label = "Show"

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, _context):
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
        layout = self.layout

        if len(unloaded_simulations(context)) > 1:
            layout.operator(SCENE_OT_Squishy_Volumes_Reload_All.bl_idname)
        for simulation in context.scene.squishy_volumes_scene.simulations:
            (header, body) = layout.panel(
                simulation.uuid, default_closed=not simulation_cache_exists(simulation)
            )
            if simulation.last_exception:
                col = header.column()
                col.alert = True
                col.label(text=f"{simulation.name}: Message")
                header.operator(
                    SCENE_OT_Squishy_Volumes_Show_Message.bl_idname
                ).uuid = simulation.uuid
            else:
                progress_text = f"{simulation.name}: "
                factor = 0.0
                if context_exists(simulation):
                    if simulation.progress_json_string:
                        progress = json.loads(simulation.progress_json_string)
                        progress_text += progress["name"]
                        completed_steps = progress["completed_steps"]
                        steps_to_completion = progress["steps_to_completion"]
                        progress_text += f" {completed_steps}/{steps_to_completion}"
                        factor = completed_steps / steps_to_completion
                    else:
                        computed = available_frames(simulation)
                        if computed == simulation.bake_frames:
                            progress_text += "Completed: "
                        else:
                            progress_text += "Paused at: "
                        progress_text += f"{computed}/{simulation.bake_frames}"
                        factor = computed / simulation.bake_frames
                else:
                    if not context_exists(simulation) and simulation_cache_locked(
                        simulation
                    ):
                        progress_text += "Cache Locked!"
                    elif simulation_cache_exists(simulation):
                        progress_text += "Cache Unloaded"
                    else:
                        progress_text += "Uninitialized"
                header.progress(text=progress_text, factor=factor)

            if body is not None:
                body.prop(simulation, "name")
                body.prop(simulation, "cache_directory")

                col = body.column()
                col.enabled = False
                col.prop(simulation, "uuid")

                col = body.column()
                col.enabled = not computing(simulation)
                col.prop(simulation, "max_giga_bytes_on_disk")

                row = body.row()
                if not context_exists(simulation) and simulation_cache_locked(
                    simulation
                ):
                    row.operator(
                        SCENE_OT_Squishy_Volumes_Remove_Lock_File.bl_idname,
                        icon="WARNING_LARGE",
                    ).uuid = simulation.uuid
                elif simulation_cache_exists(simulation):
                    tut = row.column()
                    tut.enabled = (
                        not context.scene.squishy_volumes_scene.tutorial_active
                    )
                    tut.operator(
                        SCENE_OT_Squishy_Volumes_Reload.bl_idname,
                        icon="FILE_CACHE",
                    ).uuid = simulation.uuid
                row.operator(
                    SCENE_OT_Squishy_Volumes_Remove_Simulation.bl_idname,
                    icon="TRASH",
                ).uuid = simulation.uuid

                if not context_exists(simulation):
                    continue
                json_stats = stats(simulation)
                loaded_state = json_stats["loaded_state"]
                compute = json_stats["compute"]
                bytes_on_disk = json_stats["bytes_on_disk"]

                body.label(text="Misc. Stats")
                box = body.box()
                grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
                grid.label(text="Currently used Gigabytes")
                grid.label(text=f"{bytes_on_disk * 1e-9}")

                if loaded_state is not None:
                    total_particle_count = loaded_state["total_particle_count"]
                    total_grid_node_count = loaded_state["total_grid_node_count"]
                    per_object_count = loaded_state["per_object_count"]
                    body.label(text="Loaded State Stats")
                    box = body.box()
                    grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
                    grid.label(text="Total particles + samples")
                    grid.label(text=f"{total_particle_count}")
                    grid.label(text="Total active grid nodes")
                    grid.label(text=f"{total_grid_node_count}")
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
                    grid.label(text="Approx. remaining time (sec)")
                    grid.label(text=f"{remaining_time_sec:0.2f}")
                    grid.label(text="Last frame time (sec)")
                    grid.label(text=f"{last_frame_time_sec:0.5f}")
                    grid.label(text="Last frame substeps")
                    grid.label(text=f"{last_frame_substeps}")

        tut = layout.column()
        tut.alert = (
            context.scene.squishy_volumes_scene.tutorial_active
            and not context.scene.squishy_volumes_scene.simulations
        )
        tut.operator(SCENE_OT_Squishy_Volumes_Add_Simulation.bl_idname, icon="ADD")

        if len(context.scene.squishy_volumes_scene.simulations) > 1:
            layout.separator()
            layout.prop(
                context.scene.squishy_volumes_scene,
                "selected_simulation",
                text="Select",
            )


classes = [
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
