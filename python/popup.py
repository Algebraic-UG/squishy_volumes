# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Blended MPM extension.
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


from .util import get_simulation_by_uuid

# As far as I know there isn't a way to set operator's properties
# outside of a drawing context where there is a layout
# TODO: couldn't this just be stored in the scene properties?
simulation_uuid = None


class OBJECT_OT_Blended_MPM_Popup(bpy.types.Operator):
    bl_idname = "object.blended_mpm_popup"
    bl_label = "Blended MPM Message"

    uuid: bpy.props.StringProperty()  # type: ignore

    def execute(self, _context):
        simulation = get_simulation_by_uuid(self.uuid)
        self.report(
            {"INFO"},
            message="Blended MPM clearing last message:\n" + simulation.last_exception,
        )
        simulation.last_exception = ""
        return {"FINISHED"}

    def invoke(self, context, _event):
        self.uuid = simulation_uuid
        simulation = get_simulation_by_uuid(self.uuid)
        return context.window_manager.invoke_props_dialog(
            self, title=simulation.name, confirm_text="Clear Message"
        )

    def draw(self, _context):
        simulation = get_simulation_by_uuid(self.uuid)
        for line in simulation.last_exception.splitlines():
            self.layout.label(text=line)


classes = [
    OBJECT_OT_Blended_MPM_Popup,
]


def register_popup():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_popup():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)


def popup(uuid):
    if not bpy.context.window:
        return
    global simulation_uuid
    simulation_uuid = uuid
    bpy.ops.object.blended_mpm_popup("INVOKE_DEFAULT")


def with_popup(simulation, f):
    try:
        return f()
    except RuntimeError as e:
        s = str(e)
        if simulation.last_exception != s:
            simulation.last_exception = s
            popup(simulation.uuid)
