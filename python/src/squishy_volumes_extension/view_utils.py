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
from .registration import class_registration


class OBJECT_OT_Squishy_Volumes_Restrict_View(ReportingOperator, bpy.types.Operator):
    bl_idname = "object.squishy_volumes_restrict_view"
    bl_label = "Squishy Volumes Restrict View"
    bl_description = """Add an empty cuboid for restricting the view.

The selected object is restricted via a geometry nodes modifier.
This modifier deletes points that are outside of the cuboid.
On a simulation output it comes right after loading the output."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    empty_name: bpy.props.StringProperty(
        name="Empty Name",
        description="The name of the empty used to restrict the view.",
    )
    fit_vertices: bpy.props.BoolProperty(
        name="Fit vertices",
        description="Create the empty to just fit the current geometry.",
        default=True,
    )

    @classmethod
    def poll(cls, context):
        return (
            context.active_object is not None
            and context.active_object.select_get()
            and context.active_object.type in {"MESH", "POINTCLOUD"}
        )

    def invoke(self, context, _):
        self.empty_name = f"{context.active_object.name} - View"
        return context.window_manager.invoke_props_dialog(self)

    def run(self, context):
        obj = context.active_object
        empty = api.restrict_view(
            obj, name=self.empty_name or None, fit_vertices=self.fit_vertices
        )
        self.report({"INFO"}, f"Restricting view of {obj.name} with {empty.name}")
        return {"FINISHED"}


classes = [
    OBJECT_OT_Squishy_Volumes_Restrict_View,
]


def menu_func_restrict_view(self, _context):
    self.layout.operator(
        OBJECT_OT_Squishy_Volumes_Restrict_View.bl_idname,
        icon="MODIFIER",
    )


menu_funcs = [menu_func_restrict_view]


register_view_utils, unregister_view_utils = class_registration(classes, menu_funcs)
