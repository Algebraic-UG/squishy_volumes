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

import bpy

from typing import Any

from ..properties.util import add_fields_from

from ..properties.squishy_volumes_scene import (
    get_selected_simulation,
    get_selected_input_object,
    get_selected_uuid,
)
from ..properties.squishy_volumes_object import (
    IO_NONE,
    IO_INPUT,
    get_input_objects,
)

from ..bridge import (
    SimulationInput,
    Simulation,
)
from ..input_capture import create_input_header, capture_input_frame
from ..frame_change import (
    register_handler,
    unregister_handler,
)
from ..util import (
    copy_simple_property_group,
    force_ui_redraw,
    simulation_input_exists,
    index_by_object,
    giga_f32_to_u64,
)
from ..nodes import create_geometry_nodes_generate_particles


def selection_eligible_for_input(context):
    return (
        get_selected_simulation(context.scene) is not None
        and context.active_object is not None
        and context.active_object.select_get()
        and context.active_object.type == "MESH"
        and context.active_object.squishy_volumes_object.io == IO_NONE
    )


class OBJECT_OT_Squishy_Volumes_Add_Input_Object(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_add_input_object"
    bl_label = "Add Input Object"
    bl_description = """TODO"""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return selection_eligible_for_input(context)

    def execute(self, context):
        obj = context.active_object

        simulation = get_selected_simulation(context.scene)
        obj.squishy_volumes_object.simulation_uuid = simulation.uuid
        obj.squishy_volumes_object.io = IO_INPUT

        # TODO make this configurable
        modifier = context.object.modifiers.new("Squishy Volumes Input", type="NODES")
        modifier.node_group = create_geometry_nodes_generate_particles()

        context.scene.squishy_volumes_scene.selected_input_object = index_by_object(obj)

        force_ui_redraw()

        self.report(
            {"INFO"},
            f"Added {context.object.name} to input objects of {simulation.name}.",
        )
        return {"FINISHED"}

    def invoke(self, context: bpy.types.Context, event: bpy.types.Event):
        return context.window_manager.invoke_props_dialog(self)  # ty:ignore[possibly-missing-attribute]

    def draw(self, context):
        self.layout.label(text=context.object.name)  # ty:ignore[possibly-missing-attribute]


class OBJECT_OT_Squishy_Volumes_Remove_Input_Object(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_remove_input_object"
    bl_label = "Remove"
    bl_description = """Remove the selected object from the list of inputs.

Note that this does not delete the object or remove the input modifier."""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context.scene)
        return (
            simulation is not None
            and context.active_object is not None
            and context.active_object.select_get()
            and context.active_object.squishy_volumes_object.io == IO_INPUT
            and context.active_object.squishy_volumes_object.simulation_uuid
            == simulation.uuid
        )

    def execute(self, context):
        obj = context.active_object.squishy_volumes_object
        obj.simulation_uuid = "unassigned"
        obj.io = IO_NONE
        self.report({"INFO"}, f"Removed {obj.name} from inputs.")
        return {"FINISHED"}


SIMULATION_INPUT = None


class SCENE_OT_Squishy_Volumes_Write_Input_To_Cache(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_write_input_to_cache"
    bl_label = "Write to Cache"
    bl_description = """(Over)Write the cache with the new input.

This writes global settings as well as object specific settings
to the simulation cache.

Note that this also discards all computed frames in the cache."""
    bl_options = {"REGISTER"}

    def execute(self, context):
        simulation = get_selected_simulation(context.scene)
        simulation.has_loaded_frame = False

        self.report({"INFO"}, f"Resetting {simulation.name}")

        sim = Simulation.get(uuid=simulation.uuid)
        if sim is not None:
            sim.drop()

        input_header = create_input_header(simulation)

        self.report({"INFO"}, f"Collected input header for {simulation.name}")

        global SIMULATION_INPUT
        SIMULATION_INPUT = SimulationInput.new(
            uuid=simulation.uuid,
            directory=simulation.directory,
            input_header=input_header,
            max_bytes_on_disk=giga_f32_to_u64(simulation.max_giga_bytes_on_disk),
        )

        self.report({"INFO"}, f"(Re)Created {simulation.name}")

        bpy.ops.scene.squishy_volumes_write_input_to_cache_modal("INVOKE_DEFAULT")  # ty: ignore[unresolved-attribute]
        return {"FINISHED"}

    def invoke(self, context: bpy.types.Context, event: bpy.types.Event):
        simulation = get_selected_simulation(context.scene)  # ty:ignore[invalid-argument-type]
        if simulation_input_exists(simulation):
            return context.window_manager.invoke_props_dialog(self)  # ty:ignore[possibly-missing-attribute]
        else:
            return self.execute(context)

    def draw(self, context):
        uuid = get_selected_uuid(context.scene)
        assert uuid is not None

        sim = Simulation.get(uuid=uuid)
        if sim is None:
            prior_frames = 0
        else:
            prior_frames = sim.available_frames()
            self.layout.label(text="WARNING: This is a destructive operation!")
            self.layout.label(
                text=f"The previous cache will be overwritten: {prior_frames} frames"
            )


class SCENE_OT_Squishy_Volumes_Write_Input_To_Cache_Modal(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_write_input_to_cache_modal"
    bl_label = "Write to Cache Modal"
    bl_options = set()

    _timer = None

    def invoke(self, context: bpy.types.Context, event: bpy.types.Event):
        simulation = get_selected_simulation(context.scene)  # ty:ignore[invalid-argument-type]

        context.scene.frame_set(simulation.capture_start_frame)  # ty:ignore[possibly-missing-attribute]

        self._timer = context.window_manager.event_timer_add(  # ty:ignore[possibly-missing-attribute]
            time_step=0, window=context.window
        )
        context.window_manager.progress_begin(0, simulation.capture_frames)  # ty:ignore[possibly-missing-attribute]
        context.window_manager.modal_handler_add(self)  # ty:ignore[possibly-missing-attribute]

        return {"RUNNING_MODAL"}

    def modal(self, context: bpy.types.Context, event: bpy.types.Event):
        global SIMULATION_INPUT
        assert isinstance(SIMULATION_INPUT, SimulationInput)
        simulation = get_selected_simulation(context.scene)  # ty:ignore[invalid-argument-type]

        if event.type in {"RIGHTMOUSE", "ESC"}:
            context.window_manager.event_timer_remove(self._timer)  # ty:ignore[possibly-missing-attribute, invalid-argument-type]
            SIMULATION_INPUT.drop()
            self.report(
                {"WARNING"},
                f"Capture of {simulation.name} incomplete due to user cancellation.",
            )
            return {"CANCELLED"}

        captured_frames = context.scene.frame_current - simulation.capture_start_frame  # ty:ignore[possibly-missing-attribute]
        assert captured_frames >= 0

        if captured_frames + 1 < simulation.capture_frames:
            try:
                capture_input_frame(
                    simulation=simulation,
                    simulation_input=SIMULATION_INPUT,
                )
            except RuntimeError:
                SIMULATION_INPUT.drop()
                raise

            context.scene.frame_set(context.scene.frame_current + 1)  # ty:ignore[possibly-missing-attribute]
            context.window_manager.progress_update(captured_frames)  # ty:ignore[possibly-missing-attribute]
            return {"RUNNING_MODAL"}
        context.window_manager.progress_end()  # ty:ignore[possibly-missing-attribute]

        self.report({"INFO"}, f"Finished capturing input for {simulation.name}")

        SIMULATION_INPUT = None
        sim = Simulation.new()

        if simulation.immediately_start_baking:
            sim.last_error = ""
            sim.start_compute(
                time_step=simulation.time_step,
                explicit=simulation.explicit,
                debug_mode=simulation.debug_mode,
                adaptive_time_steps=simulation.adaptive_time_steps,
                next_frame=0,
                number_of_frames=simulation.bake_frames,
                max_bytes_on_disk=giga_f32_to_u64(simulation.max_giga_bytes_on_disk),
            )
            self.report({"INFO"}, f"Commence baking of {simulation.name}.")

        return {"FINISHED"}


class SCENE_UL_Squishy_Volumes_Input_Object_List(bpy.types.UIList):
    def filter_items(self, context: bpy.types.Context, data: Any | None, property: str):
        simulation = get_selected_simulation(context.scene)  # ty:ignore[invalid-argument-type]
        if simulation is None:
            return [0] * len(bpy.data.objects), []

        input_objects = get_input_objects(simulation)
        return [
            self.bitflag_filter_item if obj in input_objects else 0
            for obj in bpy.data.objects
        ], []

    def draw_item(
        self,
        context: bpy.types.Context,
        layout: bpy.types.UILayout,
        data: Any | None,
        item: Any | None,
        icon: int | None,
        active_data: Any,
        active_property: str | None,
        index: int | None,
        flt_flag: int | None,
    ):
        assert isinstance(item, bpy.types.Object)
        layout.label(text=item.name)


class SCENE_PT_Squishy_Volumes_Input(bpy.types.Panel):
    bl_label = "Input"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"
    bl_options = set()

    @classmethod
    def poll(cls, context):
        return (
            context.mode == "OBJECT"
            and get_selected_simulation(context.scene) is not None
        )

    def draw(self, context):
        simulation = get_selected_simulation(context.scene)

        (header, body) = self.layout.panel("constants", default_closed=True)
        header.label(text="Constant Globals")
        if body is not None:
            body.prop(simulation, "grid_node_size")
            body.prop(simulation, "frames_per_second")
            body.prop(simulation, "simulation_scale")
            body.prop(simulation, "domain_min")
            body.prop(simulation, "domain_max")

        (header, body) = self.layout.panel("animatables", default_closed=True)
        header.label(text="Animatable Globals")
        if body is not None:
            body.prop(simulation, "gravity")

        row = self.layout.row()
        row.column().template_list(
            "SCENE_UL_Squishy_Volumes_Input_Object_List",
            "",
            bpy.data,
            "objects",
            context.scene.squishy_volumes_scene,
            "selected_input_object",
        )
        list_controls = row.column(align=True)
        list_controls.operator(
            OBJECT_OT_Squishy_Volumes_Add_Input_Object.bl_idname,
            text="",
            icon="ADD",
        )
        list_controls.operator(
            OBJECT_OT_Squishy_Volumes_Remove_Input_Object.bl_idname,
            text="",
            icon="REMOVE",
        )

        obj = get_selected_input_object(context.scene)
        if obj is not None:
            (header, body) = self.layout.panel(
                "input_object_settings", default_closed=True
            )
            header.label(text=f"Settings for {obj.name}")
            if body is not None:
                pass

        self.layout.prop(simulation, "capture_start_frame")
        self.layout.prop(simulation, "capture_frames")
        self.layout.separator()

        row = self.layout.row()
        row.operator(
            SCENE_OT_Squishy_Volumes_Write_Input_To_Cache.bl_idname,
            text=(
                "Overwrite Cache"
                if simulation_input_exists(simulation)
                else "Initialize Cache"
            ),
            icon="FILE_CACHE",
        )
        row.prop(simulation, "immediately_start_baking")


classes = [
    OBJECT_OT_Squishy_Volumes_Add_Input_Object,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
    SCENE_OT_Squishy_Volumes_Write_Input_To_Cache_Modal,
    SCENE_OT_Squishy_Volumes_Write_Input_To_Cache,
    SCENE_UL_Squishy_Volumes_Input_Object_List,
    SCENE_PT_Squishy_Volumes_Input,
]


def register_panel_input():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_input():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
