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

from .bridge import SimulationHandle
from .registration import class_registration
from .squishy_volumes_properties import get_simulation_objects


def popup_title(uuid: str) -> str:
    sim_objs = [
        obj
        for obj in get_simulation_objects()
        if uuid and obj.squishy_volumes.uuid == uuid
    ]
    if len(sim_objs) != 1:
        return "Squishy Volumes"
    return f"Squishy Volumes: {sim_objs[0].name}"


class SCENE_OT_Squishy_Volumes_Popup(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_popup"
    bl_label = "Squishy Volumes Message"

    uuid: bpy.props.StringProperty()
    message: bpy.props.StringProperty()
    show_allow_scaled_input: bpy.props.BoolProperty(options={"SKIP_SAVE"})

    def execute(self, context):
        sim = SimulationHandle.get(uuid=self.uuid)
        if sim is None:
            return {"FINISHED"}

        self.report(
            {"INFO"},
            message=f"Squishy Volumes clearing last message:\n{sim.last_error or ''}",
        )
        sim.last_error = None
        sim.clear_errors()
        return {"FINISHED"}

    def invoke(self, context, event):
        title = popup_title(self.uuid)
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
        if self.show_allow_scaled_input:
            self.layout.separator()
            self.layout.prop(
                context.preferences.addons[__package__].preferences,
                "allow_scaled_input",
            )


classes = [
    SCENE_OT_Squishy_Volumes_Popup,
]


register_popup, unregister_popup = class_registration(classes)


def popup(uuid: str, message: str, show_allow_scaled_input: bool = False):
    if bpy.app.background or not bpy.context.window:
        return
    bpy.ops.scene.squishy_volumes_popup(
        "INVOKE_DEFAULT",
        uuid=uuid,
        message=message,
        show_allow_scaled_input=show_allow_scaled_input,
    )


def show_message(uuid: str, message: str, show_allow_scaled_input: bool = False):
    sim = SimulationHandle.get(uuid=uuid)
    if sim is not None:
        message = f"""{message}
(Please 'Clear Message' to print to 'Info')"""
        if sim.last_error == message:
            return
        sim.last_error = message
    popup(uuid, message, show_allow_scaled_input)
