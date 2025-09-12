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
import numpy as np
import mathutils

from .util import local_bounding_box
from .nodes.geometry_nodes_restrict_view import create_geometry_nodes_restrict_view


class OBJECT_OT_Squishy_Volumes_Restrict_View(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_restrict_view"
    bl_label = "Squishy Volumes Restrict View"
    bl_description = """Add an empty cuboid for restricting the view.

The selected object is restricted via a geometry nodes modifier.
This modifier deletes vertices that are outside of the cuboid."""
    bl_options = {"REGISTER", "UNDO"}

    empty_name: bpy.props.StringProperty(
        name="Empty Name",
        description="The name of the empty used to restrict the view.",
    )  # type: ignore
    fit_vertices: bpy.props.BoolProperty(
        name="Fit vertices",
        description="Create the empty to just fit the vertices.",
        default=True,
    )  # type: ignore

    @classmethod
    def poll(cls, context):
        return (
            context.active_object is not None
            and context.active_object.select_get()
            and context.active_object.type == "MESH"
        )

    def invoke(self, context, _):
        self.empty_name = f"{context.active_object.name} - View"
        return context.window_manager.invoke_props_dialog(self)

    def execute(self, context):
        obj = context.active_object

        empty = bpy.data.objects.new(self.empty_name, None)
        empty.empty_display_type = "CUBE"

        if self.fit_vertices:
            bbox_min, bbox_max = local_bounding_box(obj)
            mid_point = (bbox_min + bbox_max) / 2
            size = bbox_max - bbox_min

            empty.matrix_world = (
                obj.matrix_world
                @ mathutils.Matrix.Translation(mid_point)
                @ mathutils.Matrix.Scale(size.x * 0.51, 4, ((1, 0, 0)))
                @ mathutils.Matrix.Scale(size.y * 0.51, 4, ((0, 1, 0)))
                @ mathutils.Matrix.Scale(size.z * 0.51, 4, ((0, 0, 1)))
            )

        context.collection.objects.link(empty)

        modifier = obj.modifiers.new("Squishy Volumes Restrict View", type="NODES")
        modifier.node_group = create_geometry_nodes_restrict_view()
        modifier["Socket_2"] = empty

        obj.modifiers.move(len(obj.modifiers) - 1, 0)

        self.report(
            {"INFO"},
            message=f"Restricting view of {obj.name} with {self.empty_name}",
        )
        return {"FINISHED"}


classes = [
    OBJECT_OT_Squishy_Volumes_Restrict_View,
]


def menu_func_restrict_view(self, _context):
    self.layout.operator(
        OBJECT_OT_Squishy_Volumes_Restrict_View.bl_idname, icon="MODIFIER"
    )


menu_funcs = [menu_func_restrict_view]


def register_view_utils():
    for cls in classes:
        bpy.utils.register_class(cls)
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.append(menu_func)


def unregister_view_utils():
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.remove(menu_func)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
