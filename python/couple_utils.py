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

from .panels.panel_output import draw_object_attributes


def selectable_simulations(_, context):
    return [
        (sim.uuid, sim.name, "")
        for sim in context.scene.squishy_volumes_scene.simulations
    ]


class OBJECT_OT_Squishy_Volumes_Recouple_Output(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_recouple_output"
    bl_label = "Squishy Volumes (Re)Couple Output"
    bl_description = """Set the selected object as an output receiver.

It is basically the same operation as adding output,
but the creation of a new object and default geometry nodes
is ommitted.

Note that this operator is somewhat difficult to use
for the initial coupling case.
(You need to lookup some strings)"""
    bl_options = {"REGISTER", "UNDO"}

    simulation: bpy.props.EnumProperty(
        items=selectable_simulations,
        name="Driving Simulation",
        description="""This is the simulation that should drive the output.""",
        options=set(),
    )  # type: ignore

    @classmethod
    def poll(cls, context):
        return (
            context.active_object is not None
            and context.active_object.select_get()
            and not context.active_object.squishy_volumes_object.simulation_uuid
        )

    def execute(self, context):
        obj = context.active_object

        obj.squishy_volumes_object.simulation_uuid = self.simulation

        self.report(
            {"INFO"},
            f"Added {obj.name} to output objects of {self.simulation}.",
        )
        return {"FINISHED"}

    def invoke(self, context, _event):
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        mpm = context.active_object.squishy_volumes_object

        self.layout.prop(self, "simulation")
        self.layout.prop(mpm, "input_name")
        self.layout.prop(mpm, "output_type")

        draw_object_attributes(self.layout, mpm.output_type, mpm.optional_attributes)


classes = [
    OBJECT_OT_Squishy_Volumes_Recouple_Output,
]


def menu_func_recouple_output(self, _context):
    self.layout.operator(
        OBJECT_OT_Squishy_Volumes_Recouple_Output.bl_idname,
        icon="MODIFIER",
    )


menu_funcs = [menu_func_recouple_output]


def register_couple_utils():
    for cls in classes:
        bpy.utils.register_class(cls)
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.append(menu_func)


def unregister_couple_utils():
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.remove(menu_func)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
