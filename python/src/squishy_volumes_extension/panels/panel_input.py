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

from ..get_preferences import get_confirm_bake_overwrite
from ..drivers import add_drivers
from ..squishy_volumes_properties import Squishy_Volumes_Properties

from ..squishy_volumes_properties import (
    get_selected_input_object,
    get_simulation_object_with_uuid,
    get_selected_simulation_uuid,
    add_fields_from,
    get_input_objects_with_uuid,
    get_selected_simulation_object,
    Squishy_Volumes_Properties_Input,
    TYPE_NONE,
    TYPE_INPUT,
    TYPE_SIMULATION,
    TYPE_OUTPUT,
    INPUT_TYPE_PARTICLES,
    INPUT_TYPE_COLLIDER,
)
from ..bridge import (
    SimulationInputHandle,
    SimulationHandle,
)
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
from ..assets import (
    create_geometry_nodes_generate_particles,
    create_geometry_nodes_generate_collider,
)


class SCENE_UL_Squishy_Volumes_Particle_Input_Object_List(bpy.types.UIList):
    def draw_item(
        self,
        context,
        layout,
        data,
        item,
        icon,
        active_data,
        active_property,
        index,
        flt_flag,
    ):
        assert isinstance(item, Squishy_Volumes_New_Input)
        row = layout.row()
        row.label(text=item.obj_name)
        if item.obj_type != "MESH":
            row.label(text="️⚠️ not a Mesh")
            return
        row.prop(item, "input_type")
        row.prop(item, "add_default_generation")


def _can_add(obj: bpy.types.ID) -> bool:
    return isinstance(obj, bpy.types.Object) and obj.type == "MESH"


def _add_input_object(operator: bpy.types.Operator, uuid: str, name: str):
    sim_obj = get_simulation_object_with_uuid(uuid)
    input_obj = bpy.data.objects[name]
    if not _can_add(input_obj):
        raise RuntimeError(f"Can't add {input_obj.name}")

    input_props = input_obj.squishy_volumes

    input_props.uuid = uuid
    input_props.type = TYPE_INPUT

    operator.report(
        {"INFO"},
        f"Added {input_obj.name} to input objects of {sim_obj.name}.",
    )

    if not input_props.add_default_generation:
        return {"FINISHED"}

    modifier = input_obj.modifiers.new("Squishy Volumes Input", type="NODES")
    if input_props.input_type == INPUT_TYPE_PARTICLES:
        modifier.node_group = create_geometry_nodes_generate_particles()
    elif input_props.input_type == INPUT_TYPE_COLLIDER:
        modifier.node_group = create_geometry_nodes_generate_collider()
    else:
        raise RuntimeError(f"Unknown input type {input_props.input_type}")

    add_drivers(sim_obj, modifier)


class SCENE_OT_Squishy_Volumes_Add_Input_Object(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_input_object"
    bl_label = "Add Input Object"
    bl_description = (
        "Adds the object with the given name to the input list of the simulation"
    )
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore
    name: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        _add_input_object(self, self.uuid, self.name)
        return {"FINISHED"}


@add_fields_from(Squishy_Volumes_Properties)
class Squishy_Volumes_New_Input(bpy.types.PropertyGroup):
    obj_name: bpy.props.StringProperty()  # type: ignore
    obj_type: bpy.props.StringProperty()  # type: ignore


class SCENE_OT_Squishy_Volumes_Add_Input_Objects(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_input_objects"
    bl_label = "Add Input Objects"
    bl_description = "Adds the *selected* objects to the input list of the simulation."
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore

    inputs: bpy.props.CollectionProperty(type=Squishy_Volumes_New_Input)  # type: ignore
    selected_input: bpy.props.IntProperty()  # type: ignore

    @classmethod
    def poll(cls, context):
        return any(obj.select_get() for obj in bpy.data.objects)

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        for input in self.inputs:
            input_obj = bpy.data.objects[input.obj_name]
            if not _can_add(input_obj):
                continue
            input_obj.squishy_volumes.input_type = input.input_type
            input_obj.squishy_volumes.add_default_generation = (
                input.add_default_generation
            )
            _add_input_object(self, self.uuid, input_obj.name)

        force_ui_redraw()
        return {"FINISHED"}

    def invoke(self, context, event):
        self.inputs.clear()
        for obj in context.selected_objects:
            input = self.inputs.add()
            input.obj_name = obj.name
            input.obj_type = obj.type
            input.type = obj.squishy_volumes.type
            input.add_default_generation = obj.squishy_volumes.add_default_generation
        return context.window_manager.invoke_props_dialog(self, width=600)

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        self.layout.template_list(
            listtype_name="SCENE_UL_Squishy_Volumes_Particle_Input_Object_List",
            list_id="",
            dataptr=self,
            propname="inputs",
            active_dataptr=self,
            active_propname="selected_input",
        )


class OBJECT_OT_Squishy_Volumes_Remove_Input_Object(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_remove_input_object"
    bl_label = "Remove"
    bl_description = """Remove the selected object from the list of inputs.

Note that this does not delete the object or remove the input modifier."""
    bl_options = {"REGISTER", "UNDO"}

    name: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        input_obj = bpy.data.objects[self.name]
        input_obj.squishy_volumes.uuid = "unassigned"
        input_obj.squishy_volumes.type = TYPE_NONE
        self.report({"INFO"}, f"Removed {input_obj.name} from inputs.")
        return {"FINISHED"}


class SCENE_UL_Squishy_Volumes_Input_Object_List(bpy.types.UIList):
    def filter_items(self, context, data, property):
        uuid = get_selected_simulation_uuid(context.scene)
        if uuid is None:
            return [0] * len(bpy.data.objects), []

        input_objects = get_input_objects_with_uuid(uuid)
        return [
            self.bitflag_filter_item if obj in input_objects else 0
            for obj in bpy.data.objects
        ], []

    def draw_item(
        self,
        context,
        layout,
        data,
        item,
        icon,
        active_data,
        active_property,
        index,
        flt_flag,
    ):
        assert isinstance(item, bpy.types.Object)
        icon = "QUESTION"
        if item.squishy_volumes.input_type == INPUT_TYPE_PARTICLES:
            icon = "OUTLINER_OB_POINTCLOUD"
        if item.squishy_volumes.input_type == INPUT_TYPE_COLLIDER:
            icon = "MOD_PHYSICS"
        layout.label(text=item.name, icon=icon)


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
            and get_selected_simulation_uuid(context.scene) is not None
        )

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        sim_obj = get_selected_simulation_object(context.scene)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

        (header, body) = self.layout.panel("constants", default_closed=True)
        header.label(text="Constant Globals")
        if body is not None:
            body.prop(sim_props, "grid_node_size")
            body.prop(sim_props, "frames_per_second")
            body.prop(sim_props, "simulation_scale")

        (header, body) = self.layout.panel("animatables", default_closed=True)
        header.label(text="Animatable Globals")
        if body is not None:
            body.prop(sim_props, "gravity")
            body.prop(sim_props, "goal_stiffness")
            body.prop(sim_props, "goal_damping")
            body.prop(sim_props, "damping")

        row = self.layout.row()
        row.column().template_list(
            "SCENE_UL_Squishy_Volumes_Input_Object_List",
            "",
            bpy.data,
            "objects",
            context.scene.squishy_volumes,
            "selected_input_object",
        )
        list_controls = row.column(align=True)
        add_input_col = list_controls.column()
        add_input_col.alert = not get_input_objects_with_uuid(sim_props.uuid)
        add_input_col.operator(
            SCENE_OT_Squishy_Volumes_Add_Input_Objects.bl_idname,
            text="",
            icon="ADD",
        ).uuid = sim_props.uuid

        remove = list_controls.column()
        remove_obj = get_selected_input_object(context.scene)
        if remove_obj is None:
            remove.enabled = False
            remove.operator(
                OBJECT_OT_Squishy_Volumes_Remove_Input_Object.bl_idname,
                text="",
                icon="REMOVE",
            )
        else:
            remove.operator(
                OBJECT_OT_Squishy_Volumes_Remove_Input_Object.bl_idname,
                text="",
                icon="REMOVE",
            ).name = remove_obj.name


classes = [
    Squishy_Volumes_New_Input,
    SCENE_UL_Squishy_Volumes_Particle_Input_Object_List,
    SCENE_OT_Squishy_Volumes_Add_Input_Object,
    SCENE_OT_Squishy_Volumes_Add_Input_Objects,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
    SCENE_UL_Squishy_Volumes_Input_Object_List,
    SCENE_PT_Squishy_Volumes_Input,
]


def register_panel_input():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_input():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
