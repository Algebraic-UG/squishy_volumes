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

from ..properties.util import (
    add_fields_from,
    get_input_objects,
    get_selected_input_object,
    get_selected_simulation,
    get_simulation_specific_settings,
    get_selected_input_object_and_settings,
)
from ..properties.squishy_volumes_object_settings import (
    Squishy_Volumes_Object_Settings,
)
from ..bridge import (
    available_frames,
    context_exists,
    new_simulation,
    record_input,
    start_compute,
)
from ..setup import create_setup_json
from ..frame_change import (
    register_frame_handler,
    unregister_frame_handler,
)
from ..util import (
    copy_simple_property_group,
    force_ui_redraw,
    simulation_cache_exists,
)
from ..popup import with_popup
from ..nodes import create_geometry_nodes_generate_particles


def selection_eligible_for_input(context):
    return (
        get_selected_simulation(context) is not None
        and context.active_object is not None
        and context.active_object.select_get()
        and context.active_object.type == "MESH"
        # This could be allowed?
        and not context.active_object.squishy_volumes_object.simulation_uuid
        and not get_simulation_specific_settings(
            get_selected_simulation(context), context.active_object
        )
    )


@add_fields_from(Squishy_Volumes_Object_Settings)
class OBJECT_OT_Squishy_Volumes_Add_Input_Object(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_add_input_object"
    bl_label = "Add Input Object"
    bl_description = """TODO"""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return selection_eligible_for_input(context)

    def execute(self, context):
        settings = (
            context.object.squishy_volumes_object.simulation_specific_settings.add()
        )
        copy_simple_property_group(self, settings)

        simulation = get_selected_simulation(context)
        settings.simulation_uuid = simulation.uuid

        # TODO make this configurable
        modifier = context.object.modifiers.new("Squishy Volumes Input", type="NODES")
        modifier.node_group = create_geometry_nodes_generate_particles()

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
        self.layout.prop(self, "object_enum")  # ty:ignore[possibly-missing-attribute]


class OBJECT_OT_Squishy_Volumes_Remove_Input_Object(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_remove_input_object"
    bl_label = "Remove"
    bl_description = """Remove the selected object from the list of inputs.

Note that this does not delete the object or remove the input modifier."""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return (
            context.mode == "OBJECT"
            and get_selected_simulation(context) is not None
            and get_selected_input_object(context) is not None
        )

    def execute(self, context):
        simulation = get_selected_simulation(context)
        obj = get_selected_input_object(context)
        simulation_specific_settings = (
            obj.squishy_volumes_object.simulation_specific_settings
        )
        simulation_specific_settings.remove(
            next(
                idx
                for idx, settings in enumerate(simulation_specific_settings)
                if settings.simulation_uuid == simulation.uuid
            )
        )
        self.report(
            {"INFO"}, f"Removed {obj.name} from input objects of {simulation.name}."
        )
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Write_Input_To_Cache(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_write_input_to_cache"
    bl_label = "Write to Cache"
    bl_description = """(Over)Write the cache with the new input.

This writes global settings as well as object specific settings
to the simulation cache.

Note that this also discards all computed frames in the cache."""
    bl_options = {"REGISTER"}

    def execute(self, context):
        simulation = get_selected_simulation(context)

        setup_json = with_popup(simulation, lambda: create_setup_json(simulation))

        if not with_popup(simulation, lambda: new_simulation(simulation, setup_json)):
            return {"FINISHED"}

        unregister_frame_handler()
        frame_current = context.scene.frame_current

        context.scene.frame_set(frame_current)
        register_frame_handler()

        if not setup_json:
            return {"CANCELLED"}

        simulation.last_exception = ""
        simulation.loaded_frame = -1

        record_input(simulation, 0, bulk)

        self.report({"INFO"}, f"Updating cache of {simulation.name}")

        if simulation.immediately_start_baking:
            simulation.last_exception = ""
            start_compute(simulation, available_frames(simulation))
            self.report({"INFO"}, f"Commence baking of {simulation.name}.")

        return {"FINISHED"}

    def invoke(self, context: bpy.types.Context, event: bpy.types.Event):
        return context.window_manager.invoke_props_dialog(self)  # ty:ignore[possibly-missing-attribute]

    def draw(self, context):
        simulation = get_selected_simulation(context)
        if simulation_cache_exists(simulation):
            self.layout.label(text="WARNING: This is a destructive operation!")
            self.layout.label(
                text=f"The previous cache will be overwritten: {available_frames(simulation)} frames"
            )


class SCENE_UL_Squishy_Volumes_Input_Object_List(bpy.types.UIList):
    def filter_items(self, context: bpy.types.Context, data: Any | None, property: str):
        simulation = get_selected_simulation(context)
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
        return context.mode == "OBJECT" and get_selected_simulation(context) is not None

    def draw(self, context):
        simulation = get_selected_simulation(context)

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

        obj_and_settings = get_selected_input_object_and_settings(context)
        if obj_and_settings is not None:
            obj, settings = obj_and_settings
            (header, body) = self.layout.panel(
                "input_object_settings", default_closed=True
            )
            header.label(text=f"Settings for {obj.name}")
            if body is not None:
                body.prop(settings, "object_enum")

        self.layout.prop(simulation, "capture_start_frame")
        self.layout.prop(simulation, "capture_frames")
        self.layout.separator()

        row = self.layout.row()
        row.operator(
            SCENE_OT_Squishy_Volumes_Write_Input_To_Cache.bl_idname,
            text=(
                "Overwrite Cache"
                if simulation_cache_exists(simulation)
                else "Initialize Cache"
            ),
            icon="FILE_CACHE",
        )
        row.prop(simulation, "immediately_start_baking")


classes = [
    OBJECT_OT_Squishy_Volumes_Add_Input_Object,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
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
