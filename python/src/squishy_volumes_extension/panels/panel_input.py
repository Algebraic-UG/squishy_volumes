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

from ..preferences import get_confirm_bake_overwrite
from ..nodes.drivers import add_drivers

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
from ..nodes import (
    create_geometry_nodes_generate_particles,
    create_geometry_nodes_generate_goal_positions,
    create_geometry_nodes_generate_collider,
)


class SCENE_UL_Squishy_Volumes_Particle_Input_Object_List(bpy.types.UIList):
    def filter_items(self, context, data, property):
        return [
            self.bitflag_filter_item if obj.select_get() else 0
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
        row = layout.row()
        row.label(text=item.name)
        if item.type != "MESH":
            row.label(text="️⚠️ not a Mesh")
            return
        if item.squishy_volumes.type == TYPE_SIMULATION:
            row.label(text="⚠️ already a simulation")
            return
        if item.squishy_volumes.type == TYPE_INPUT:
            row.label(text="⚠️ already an input")
            return
        if item.squishy_volumes.type == TYPE_OUTPUT:
            row.label(text="⚠️ already an output")
            return
        row.prop(item.squishy_volumes, "input_type")
        row.prop(item.squishy_volumes, "add_default_generation")


def _can_add(obj: bpy.types.ID) -> bool:
    return (
        isinstance(obj, bpy.types.Object)
        and obj.type == "MESH"
        and obj.squishy_volumes.type == TYPE_NONE  # ty:ignore[unresolved-attribute]
    )


class SCENE_OT_Squishy_Volumes_Add_Input_Object(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_input_object"
    bl_label = "Add Input Object"
    bl_description = """TODO"""
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore
    name: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        input_obj = bpy.data.objects[self.name]
        if not _can_add(input_obj):
            raise RuntimeError(f"Can't add {input_obj.name}")

        input_props = input_obj.squishy_volumes
        if input_props.input_type == INPUT_TYPE_PARTICLES:
            node_group_generate_particles = create_geometry_nodes_generate_particles()
        elif input_props.input_type == INPUT_TYPE_COLLIDER:
            node_group_generate_collider = create_geometry_nodes_generate_collider()
        else:
            raise RuntimeError(f"Unknown input type {input_props.input_type}")

        input_props.uuid = self.uuid
        input_props.type = TYPE_INPUT

        self.report(
            {"INFO"},
            f"Added {input_obj.name} to input objects of {sim_obj.name}.",
        )

        if not input_props.add_default_generation:
            return {"FINISHED"}

        modifier = input_obj.modifiers.new("Squishy Volumes Input", type="NODES")
        if input_props.input_type == INPUT_TYPE_PARTICLES:
            modifier.node_group = node_group_generate_particles
        if input_props.input_type == INPUT_TYPE_COLLIDER:
            modifier.node_group = node_group_generate_collider

        add_drivers(sim_obj, modifier)

        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Add_Input_Objects(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_input_objects"
    bl_label = "Add Input Objects"
    bl_description = """TODO"""
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore
    selected_active: bpy.props.IntProperty()  # type: ignore

    @classmethod
    def poll(cls, context):
        return any(obj.select_get() for obj in bpy.data.objects)

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        for input_obj in bpy.data.objects:
            if not input_obj.select_get() or not _can_add(input_obj):
                continue
            bpy.ops.scene.squishy_volumes_add_input_object(  # ty:ignore[unresolved-attribute]
                "INVOKE_DEFAULT", uuid=self.uuid, name=input_obj.name
            )

        force_ui_redraw()
        return {"FINISHED"}

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self, width=600)

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        self.layout.template_list(
            listtype_name="SCENE_UL_Squishy_Volumes_Particle_Input_Object_List",
            list_id="",
            dataptr=bpy.data,
            propname="objects",
            active_dataptr=self,
            active_propname="selected_active",
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
        input_obj.squishy_volumes.uuid = "unassigned"  # ty:ignore[unresolved-attribute]
        input_obj.squishy_volumes.type = TYPE_NONE  # ty:ignore[unresolved-attribute]
        self.report({"INFO"}, f"Removed {input_obj.name} from inputs.")  # ty:ignore[unresolved-reference]
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
        layout.label(text=item.name)


# TODO: re-add goals
## TODO: this doesn't feel like it's the right place... the whole file has become somewhat bloated
# class OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals(bpy.types.Operator):
#    bl_idname = "object.squishy_volumes_input_object_add_goals"
#    bl_label = "Add Goals"
#    bl_description = """TODO"""
#    bl_options = {"REGISTER", "UNDO"}
#
#    @classmethod
#    def poll(cls, context):
#        return get_selected_input_object(context.scene) is not None
#
#    def execute(self, context):
#        obj = get_selected_input_object(context.scene)
#
#        node_group = create_geometry_nodes_generate_goal_positions()
#        modifier = obj.modifiers.new("Squishy Volumes Goals", type="NODES")
#        modifier.node_group = node_group
#
#        bpy.ops.mesh.primitive_ico_sphere_add()
#        choose = context.active_object
#        choose.name = f"{obj.name} - Choose"
#
#        move = bpy.data.objects.new(f"{obj.name} - Move", None)
#        context.collection.objects.link(move)
#
#        move.parent = choose
#
#        modifier["Socket_2"] = choose
#        modifier["Socket_3"] = move
#
#        obj.update_tag()
#        context.view_layer.update()
#
#        self.report({"INFO"}, f"Added goals to {obj.name}.")
#        return {"FINISHED"}


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
        list_controls.operator(
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
    SCENE_UL_Squishy_Volumes_Particle_Input_Object_List,
    SCENE_OT_Squishy_Volumes_Add_Input_Object,
    SCENE_OT_Squishy_Volumes_Add_Input_Objects,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
    SCENE_UL_Squishy_Volumes_Input_Object_List,
    # TODO
    # OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals,
    SCENE_PT_Squishy_Volumes_Input,
]


# TODO
# def menu_func_add_goals(self, _context):
#    self.layout.operator(
#        OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals.bl_idname,
#        icon="MODIFIER",
#    )
#
#
# menu_funcs = [menu_func_add_goals]


def register_panel_input():
    for cls in classes:
        bpy.utils.register_class(cls)


#    for menu_func in menu_funcs:
#        bpy.types.VIEW3D_MT_object.append(menu_func)


def unregister_panel_input():
    #    for menu_func in menu_funcs:
    #        bpy.types.VIEW3D_MT_object.remove(menu_func)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
