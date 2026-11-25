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

from ..get_preferences import first_run_setup_pending, get_preferences
from ..operators.input_output import (
    OBJECT_OT_Squishy_Volumes_Copy_Alembic_Attributes,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
    SCENE_OT_Squishy_Volumes_Add_Input_Objects,
)
from ..properties import (
    INPUT_TYPE_COLLIDER,
    INPUT_TYPE_PARTICLES,
    Squishy_Volumes_Alembic_Attribute,
    get_input_objects_with_uuid,
    get_selected_input_object,
    get_selected_simulation_object,
    get_selected_simulation_uuid,
)
from ..registration import class_registration


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


class SCENE_UL_Squishy_Volumes_Alembic_Attribute_List(bpy.types.UIList):
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
        assert isinstance(item, Squishy_Volumes_Alembic_Attribute)
        row = layout.row()
        row.label(text=item.name)
        row.prop(item, "enabled", text="")


class SCENE_PT_Squishy_Volumes_Input_Output(bpy.types.Panel):
    bl_label = "Input / Output"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"
    bl_options = set()  # noqa: RUF012

    @classmethod
    def poll(cls, context):
        return (
            context.mode == "OBJECT"
            and not first_run_setup_pending(get_preferences())
            and get_selected_simulation_uuid(context.scene) is not None
        )

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        sim_obj = get_selected_simulation_object(context.scene)
        assert sim_obj is not None
        sim_props = sim_obj.squishy_volumes

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
        input_objects = row.column()
        input_objects.label(text="Input Objects")
        input_objects_list = input_objects.row()
        input_objects_list.template_list(
            "SCENE_UL_Squishy_Volumes_Input_Object_List",
            "",
            bpy.data,
            "objects",
            context.scene.squishy_volumes,
            "selected_input_object",
        )
        list_controls = input_objects_list.column(align=True)
        add_input_col = list_controls.column()
        add_input_col.alert = not get_input_objects_with_uuid(sim_props.uuid)
        add_input_col.operator(
            SCENE_OT_Squishy_Volumes_Add_Input_Objects.bl_idname,
            text="",
            icon="ADD",
        ).uuid = sim_props.uuid

        remove = list_controls.column()
        selected_input_obj = get_selected_input_object(context.scene)
        if selected_input_obj is None:
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
            ).name = selected_input_obj.name

        if selected_input_obj is None:
            return

        attribute_list = row.column()
        attribute_header = attribute_list.row()
        attribute_header.label(text="Output Attr.")
        attribute_header.operator(
            OBJECT_OT_Squishy_Volumes_Copy_Alembic_Attributes.bl_idname,
            text="",
            icon="DUPLICATE",
        )
        attribute_list.template_list(
            listtype_name="SCENE_UL_Squishy_Volumes_Alembic_Attribute_List",
            list_id=selected_input_obj.name,
            dataptr=selected_input_obj.squishy_volumes,
            propname="alembic_attributes",
            active_dataptr=selected_input_obj.squishy_volumes,
            active_propname="selected_alembic_attribute",
        )


classes = [
    SCENE_UL_Squishy_Volumes_Input_Object_List,
    SCENE_UL_Squishy_Volumes_Alembic_Attribute_List,
    SCENE_PT_Squishy_Volumes_Input_Output,
]


register_panel_input_output, unregister_panel_input_output = class_registration(classes)
