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

from .bridge import available_gpus

_DETECTED_GPUS = []


class SCENE_OT_Squishy_Volumes_Scan_GPUs(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_scan_gpus"
    bl_label = "Scan For GPUs"
    bl_description = """Detects the GPUs on your system
and makes them available in the addon preferences."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        global _DETECTED_GPUS
        _DETECTED_GPUS = available_gpus()
        return {"FINISHED"}


def _get_detected_gpus(_preferences, _context):
    return [(gpu, gpu, gpu) for gpu in _DETECTED_GPUS]


class SquishyVolumesPreferences(bpy.types.AddonPreferences):
    bl_idname = __package__

    gpu: bpy.props.EnumProperty(
        items=_get_detected_gpus,
        name="GPU for Compute",
        description="This GPU is used by Squishy Volumes to run your simulations.",
        options=set(),
    )  # type: ignore

    confirm_bake_overwrite: bpy.props.BoolProperty(
        name="Confirm Overwrite",
        description="""Each time the simulation input is (over)written, old frames are discarded.

To prevent accidental deletion a popup adds a manual confirmation step:
'WARNING: This is a destructive operation!'

Disable this option to skip that popup.""",
        default=True,
    )  # type: ignore

    domain_min: bpy.props.FloatVectorProperty(
        name="Domain Min",
        description="""The min corner of the domain AABB.
Particles that fall below this are deactivated.""",
        default=(-100.0, -100.0, -100.0),
        options=set(),  # can't be animated
    )  # type: ignore
    domain_max: bpy.props.FloatVectorProperty(
        name="Domain Max",
        description="""The max corner of the domain AABB
Particles that rise above this are deactivated.""",
        default=(100.0, 100.0, 100.0),
        options=set(),  # can't be animated
    )  # type: ignore

    print_debug_info: bpy.props.BoolProperty(
        name="Print Debug Info",
        description="""Can be used to disable certain debug printouts.
This is most likely only relevant to developers of other extensions.""",
        default=True,
        options=set(),  # can't be animated
    )  # type: ignore

    def draw(self, context: bpy.types.Context) -> None:
        self.layout.operator(SCENE_OT_Squishy_Volumes_Scan_GPUs.bl_idname)
        self.layout.prop(self, "gpu")
        self.layout.prop(self, "confirm_bake_overwrite")
        self.layout.prop(self, "domain_min")
        self.layout.prop(self, "domain_max")
        self.layout.prop(self, "print_debug_info")


def register_preferences():
    bpy.utils.register_class(SCENE_OT_Squishy_Volumes_Scan_GPUs)
    bpy.utils.register_class(SquishyVolumesPreferences)


def unregister_preferences():
    bpy.utils.unregister_class(SquishyVolumesPreferences)
    bpy.utils.unregister_class(SCENE_OT_Squishy_Volumes_Scan_GPUs)
