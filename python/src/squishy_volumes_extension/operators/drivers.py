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
from ..registration import class_registration
from .operator_base import ReportingOperator


class OBJECT_OT_Squishy_Volumes_Input_Object_Add_Drivers(
    ReportingOperator, bpy.types.Operator
):
    bl_idname = "object.squishy_volumes_add_drivers"
    bl_label = "Squishy Volumes Add Drivers"
    bl_description = """This adds drivers to the objects' modifiers' sockets.

Only changes selected objects' modifiers' sockets.
Only changes modifiers starting with 'Squishy Volumes'.

This connects the respective value, for example, 'Grid Node Size', with the
value of the respective simulation(s) the objects are an input of."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    def run(self, context):
        for obj in context.selected_objects:
            if not api.is_input(obj):
                self.report({"INFO"}, f"Ignoring {obj.name}, it isn't an input.")
                continue
            if not obj.is_editable:
                self.report({"INFO"}, f"Ignoring {obj.name}, it is linked.")
                continue
            driven = api.add_drivers(obj)
            names = ", ".join(modifier.name for modifier in driven)
            self.report({"INFO"}, f"Added drivers to {obj.name}: {names or 'none'}.")
        return {"FINISHED"}


classes = [
    OBJECT_OT_Squishy_Volumes_Input_Object_Add_Drivers,
]


def menu_func_add_drivers(self, _context):
    self.layout.operator(
        OBJECT_OT_Squishy_Volumes_Input_Object_Add_Drivers.bl_idname,
        icon="MODIFIER",
    )


menu_funcs = [menu_func_add_drivers]


register_drivers, unregister_drivers = class_registration(classes, menu_funcs)
