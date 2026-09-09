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

import bpy  # ty: ignore[unresolved-import]

from .bridge import SimulationHandle
from .squishy_volumes_properties import get_simulation_object_with_uuid


class SCENE_OT_Squishy_Volumes_Popup(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_popup"
    bl_label = "Squishy Volumes Message"

    uuid: bpy.props.StringProperty()  # type: ignore
    message: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        sim = SimulationHandle.get(uuid=self.uuid)
        if sim is None:
            return {"FINISHED"}

        self.report(
            {"INFO"},
            message="Squishy Volumes clearing last message:\n" + sim.last_error,
        )
        sim.last_error = None
        return {"FINISHED"}

    def invoke(self, context, event):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        title = f"Squishy Volumes: {sim_obj.name}"
        if SimulationHandle.exists(uuid=self.uuid):
            return context.window_manager.invoke_props_dialog(
                self,
                title=title,
                confirm_text="Clear Message",
            )
        return context.window_manager.invoke_props_dialog(self, title=title, width=600)

    def draw(self, context):
        assert self.layout is not None
        for line in self.message.splitlines():
            self.layout.label(text=line)


classes = [
    SCENE_OT_Squishy_Volumes_Popup,
]


def register_popup():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_popup():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)


def popup(uuid: str, message: str):
    if not bpy.context.window:
        return
    bpy.ops.scene.squishy_volumes_popup("INVOKE_DEFAULT", uuid=uuid, message=message)


def with_popup(*, uuid, f):
    try:
        return f()
    except RuntimeError as e:
        sim = SimulationHandle.get(uuid=uuid)
        if sim is None:
            message = f"{e}"
        else:
            message = f"""{e}
(Please 'Clear Message' to print to 'Info')"""
            if sim.last_error == message:
                return
            sim.last_error = message

        popup(uuid, message)
