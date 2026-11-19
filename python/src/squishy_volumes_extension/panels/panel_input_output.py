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

from .. import api
from ..operator_base import ReportingOperator, SimulationOperator
from ..registration import class_registration
from ..squishy_volumes_properties import (
    INPUT_TYPE_COLLIDER,
    INPUT_TYPE_PARTICLES,
    Squishy_Volumes_Alembic_Attribute,
    Squishy_Volumes_Properties_Input,
    add_fields_from,
    get_input_objects_with_uuid,
    get_selected_input_object,
    get_selected_simulation_object,
    get_selected_simulation_uuid,
)
from ..util import force_ui_redraw


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
        if item.problem:
            row.label(text=item.problem, icon="ERROR")
            return
        row.prop(item, "input_type")
        row.prop(item, "add_default_generation")
        col = row.column()
        col.enabled = item.input_type == INPUT_TYPE_PARTICLES and item.add_output_object
        col.prop(item, "add_default_visualization")
        col = row.column()
        col.enabled = item.input_type == INPUT_TYPE_PARTICLES
        col.prop(item, "add_output_object")


@add_fields_from(Squishy_Volumes_Properties_Input)
class Squishy_Volumes_New_Input(bpy.types.PropertyGroup):
    obj_name: bpy.props.StringProperty()
    problem: bpy.props.StringProperty()


class SCENE_OT_Squishy_Volumes_Add_Input_Objects(
    SimulationOperator, bpy.types.Operator
):
    bl_idname = "scene.squishy_volumes_add_input_objects"
    bl_label = "Add Input Objects"
    bl_description = "Adds the *selected* objects to the input list of the simulation."
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    inputs: bpy.props.CollectionProperty(type=Squishy_Volumes_New_Input)
    selected_input: bpy.props.IntProperty()

    @classmethod
    def poll(cls, context):
        return any(obj.select_get() for obj in bpy.data.objects)

    def run(self, context):
        sim_obj = self.sim_obj
        added = []
        for new_input in self.inputs:
            obj = bpy.data.objects[new_input.obj_name]
            if api.input_problem(obj) is not None:
                continue
            api.add_input(
                sim_obj,
                obj,
                new_input.input_type,
                default_generation=new_input.add_default_generation,
                output_object=new_input.add_output_object,
                default_visualization=new_input.add_default_visualization,
            )
            added.append(obj.name)
        force_ui_redraw()
        self.report({"INFO"}, f"Added {', '.join(added)} to {sim_obj.name}.")
        return {"FINISHED"}

    def invoke(self, context, event):
        self.inputs.clear()
        for obj in context.selected_objects:
            new_input = self.inputs.add()
            new_input.obj_name = obj.name
            new_input.problem = api.input_problem(obj) or ""
            new_input.input_type = obj.squishy_volumes.input_type
            new_input.add_default_generation = (
                obj.squishy_volumes.add_default_generation
            )
            new_input.add_output_object = obj.squishy_volumes.add_output_object
            new_input.add_default_visualization = (
                obj.squishy_volumes.add_default_visualization
            )
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


class OBJECT_OT_Squishy_Volumes_Remove_Input_Object(
    ReportingOperator, bpy.types.Operator
):
    bl_idname = "object.squishy_volumes_remove_input_object"
    bl_label = "Remove"
    bl_description = """Remove the selected object from the list of inputs.

Note that this does not delete the object or remove the input modifier."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    name: bpy.props.StringProperty()

    def run(self, context):
        input_obj = bpy.data.objects.get(self.name)
        if input_obj is None:
            raise api.InvalidInput(f"There is no object named '{self.name}'")
        api.remove_input(input_obj)
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


class OBJECT_OT_Squishy_Volumes_Copy_Alembic_Attributes(
    ReportingOperator, bpy.types.Operator
):
    bl_idname = "object.squishy_volumes_copy_alembic_attributes"
    bl_label = "Copy"
    bl_description = """Copy the alembic attribute selection from the active object to the selected ones.

Note that this only takes affect in the cache after input recording."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    @classmethod
    def poll(cls, context):
        return hasattr(context, "active_object") and context.active_object is not None

    def run(self, context):
        assert context.active_object is not None
        updated = api.copy_alembic_attributes(
            context.active_object, context.selected_objects
        )
        self.report({"INFO"}, f"Copied attributes to {len(updated)} object(s).")
        return {"FINISHED"}


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
    Squishy_Volumes_New_Input,
    SCENE_UL_Squishy_Volumes_Particle_Input_Object_List,
    SCENE_OT_Squishy_Volumes_Add_Input_Objects,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
    SCENE_UL_Squishy_Volumes_Input_Object_List,
    OBJECT_OT_Squishy_Volumes_Copy_Alembic_Attributes,
    SCENE_UL_Squishy_Volumes_Alembic_Attribute_List,
    SCENE_PT_Squishy_Volumes_Input_Output,
]


register_panel_input_output, unregister_panel_input_output = class_registration(classes)
