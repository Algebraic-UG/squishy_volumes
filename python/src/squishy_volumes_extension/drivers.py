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

from .squishy_volumes_properties import (
    TYPE_INPUT,
    TYPE_OUTPUT,
    get_simulation_object_with_uuid,
)


def add_drivers(sim_obj, modifier):
    tree = modifier.node_group.interface.items_tree
    if "Grid Node Size" in tree:
        identifier = tree["Grid Node Size"].identifier

        if bpy.app.version[0] == 5 and bpy.app.version[1] < 2:
            driver = modifier.driver_add(f'["{identifier}"]').driver
        else:
            driver = (
                getattr(modifier.properties.inputs, identifier)
                .driver_add("value")
                .driver
            )

        driver.expression = "grid_node_size"
        var = driver.variables.new()
        var.name = "grid_node_size"
        var.type = "SINGLE_PROP"
        target = var.targets[0]
        target.fallback_value = 1
        target.data_path = "squishy_volumes.grid_node_size"
        target.id_type = "OBJECT"
        target.id = sim_obj


class OBJECT_OT_Squishy_Volumes_Input_Object_Add_Drivers(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_add_drivers"
    bl_label = "Add Drivers"
    bl_description = """This adds drivers to the objects' modifiers' sockets.

Only changes active input/output objects.
Only changes modifiers starting with 'Squishy Volmues'.

This connects the respective value, for example, 'Grid Node Size', with the
value of the respective simulation(s) the objects are an output of."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        for obj in context.selected_objects:
            props = obj.squishy_volumes
            if props.type not in [TYPE_OUTPUT, TYPE_INPUT]:
                continue

            sim_obj = get_simulation_object_with_uuid(props.uuid)
            if sim_obj is None:
                continue

            for modifier in obj.modifiers:
                if not modifier.name.startswith("Squishy Volumes"):
                    continue
                add_drivers(sim_obj, modifier)

            self.report(
                {"INFO"}, f"Added driver to modifier {modifier.name} of {obj.name}."
            )

        return {"FINISHED"}


classes = [
    OBJECT_OT_Squishy_Volumes_Input_Object_Add_Drivers,
]


def menu_func_add_goals(self, _context):
    self.layout.operator(
        OBJECT_OT_Squishy_Volumes_Input_Object_Add_Drivers.bl_idname,
        icon="MODIFIER",
    )


menu_funcs = [menu_func_add_goals]


def register_drivers():
    for cls in classes:
        bpy.utils.register_class(cls)

    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.append(menu_func)


def unregister_drivers():
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.remove(menu_func)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
