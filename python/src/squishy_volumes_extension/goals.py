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


from .squishy_volumes_properties import get_selected_input_object
from .nodes import create_geometry_nodes_generate_goal_positions


class OBJECT_OT_Squishy_Volumes_Input_Object_Add_Goals(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_input_object_add_goals"
    bl_label = "Add Goal Control Objects"
    bl_description = """This adds two objects and a modifier to control particles:
A 'choose' and a 'move' object, and the 'set goals' modifier.

The choose object is a mesh object to choose affected particles.
It can be any mesh and the chosen particles are the ones inside the mesh.

The move object is an empty object that moves the chosen particles.
Both objects can be animated!

The modifier causes the simulation to record the resulting 'goal' positions
and forces particles to move towards them."""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return get_selected_input_object(context.scene) is not None

    def execute(self, context):
        obj = get_selected_input_object(context.scene)
        assert obj is not None

        node_group = create_geometry_nodes_generate_goal_positions()
        modifier = obj.modifiers.new("Squishy Volumes Goals", type="NODES")
        modifier.node_group = node_group  # ty:ignore[unresolved-attribute]

        bpy.ops.mesh.primitive_ico_sphere_add()
        choose = context.active_object
        choose.name = f"{obj.name} - Choose"

        move = bpy.data.objects.new(f"{obj.name} - Move", None)
        context.collection.objects.link(move)

        move.parent = choose

        modifier["Socket_2"] = choose
        modifier["Socket_3"] = move

        obj.update_tag()
        context.view_layer.update()

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


def register_goals():
    for cls in classes:
        bpy.utils.register_class(cls)

    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.append(menu_func)


def unregister_goals():
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.remove(menu_func)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
