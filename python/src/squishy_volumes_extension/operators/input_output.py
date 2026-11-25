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
from ..properties import (
    INPUT_TYPE_PARTICLES,
    Squishy_Volumes_Properties_Input,
    add_fields_from,
)
from ..registration import class_registration
from ..util import force_ui_redraw
from .operator_base import ReportingOperator, SimulationOperator


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


def named_object(name: str) -> bpy.types.Object:
    obj = bpy.data.objects.get(name)
    if obj is None:
        raise api.InvalidInput(f"There is no object named '{name}'")
    return obj


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
        objects = [named_object(new_input.obj_name) for new_input in self.inputs]
        added = []
        for obj, new_input in zip(objects, self.inputs, strict=True):
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
        input_obj = named_object(self.name)
        api.remove_input(input_obj)
        self.report({"INFO"}, f"Removed {input_obj.name} from inputs.")
        return {"FINISHED"}


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


classes = [
    Squishy_Volumes_New_Input,
    SCENE_UL_Squishy_Volumes_Particle_Input_Object_List,
    SCENE_OT_Squishy_Volumes_Add_Input_Objects,
    OBJECT_OT_Squishy_Volumes_Remove_Input_Object,
    OBJECT_OT_Squishy_Volumes_Copy_Alembic_Attributes,
]


register_input_output_operators, unregister_input_output_operators = class_registration(
    classes
)
