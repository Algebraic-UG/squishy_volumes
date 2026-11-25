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

from . import api
from .operator_base import ReportingOperator
from .properties import get_selected_input_object
from .registration import class_registration


class OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals(
    ReportingOperator, bpy.types.Operator
):
    bl_idname = "object.squishy_volumes_input_object_add_goals"
    bl_label = "Squishy Volumes Add Goal Control Objects"
    bl_description = """This adds two objects and a modifier to control particles:
A 'choose' and a 'move' object, and the 'set goals' modifier.

The choose object is a mesh object to choose affected particles.
It can be any mesh and the chosen particles are the ones inside the mesh.

The move object is an empty object that moves the chosen particles.
Both objects can be animated!

The modifier causes the simulation to record the resulting 'goal' positions
and forces particles to move towards them."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    @classmethod
    def poll(cls, context):
        return get_selected_input_object(context.scene) is not None

    def run(self, context):
        obj = get_selected_input_object(context.scene)
        if obj is None:
            raise api.InvalidInput("Select an input object to add goals to")
        choose, _move = api.add_goals(obj, location=context.scene.cursor.location)
        for selected in context.selected_objects:
            selected.select_set(False)
        choose.select_set(True)
        context.view_layer.objects.active = choose
        self.report({"INFO"}, f"Added goals to {obj.name}.")
        return {"FINISHED"}


classes = [
    OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals,
]


def menu_func_add_goals(self, _context):
    self.layout.operator(
        OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals.bl_idname,
        icon="MODIFIER",
    )


menu_funcs = [menu_func_add_goals]


register_goals, unregister_goals = class_registration(classes, menu_funcs)
