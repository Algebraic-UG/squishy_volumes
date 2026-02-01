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
import time
import timeit

from .bridge import computing
from .properties.squishy_volumes_scene import get_simulation_by_uuid


class SCENE_OT_Squishy_Volumes_Wait_Until_Finished(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_wait_until_finished"
    bl_label = "Block Until Finished or Timeout"
    bl_description = """Block and poll the given simulation.
Loop with small sleep and return once finished the simulation or timeout.
This is only useful for scripting."""
    bl_options = {"REGISTER"}

    simulation_uuid: bpy.props.StringProperty(name="Simulation UUID")  # type: ignore
    timeout_sec: bpy.props.FloatProperty(name="Timeout", min=0.0)  # type: ignore

    def execute(self, context):
        start = timeit.timeit()

        simulation = get_simulation_by_uuid(context.scene, self.simulation_uuid)

        while computing(simulation):
            if (timeit.timeit() - start) > self.timeout_sec:
                raise RuntimeError("Timed out")
            time.sleep(0.01)
        self.report({"INFO"}, f"Simulation no longer computing: {simulation.name}")
        return {"FINISHED"}


classes = [
    SCENE_OT_Squishy_Volumes_Wait_Until_Finished,
]


def register_script_utils():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_script_utils():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
