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
import mathutils


class SquishyVolumesPreferences(bpy.types.AddonPreferences):
    bl_idname = __package__

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

    def draw(self, context: bpy.types.Context) -> None:
        self.layout.prop(self, "confirm_bake_overwrite")
        self.layout.prop(self, "domain_min")
        self.layout.prop(self, "domain_max")


def get_confirm_bake_overwrite() -> bool:
    return bpy.context.preferences.addons.get(  # ty:ignore[possibly-missing-attribute]
        __package__  # ty:ignore[invalid-argument-type]
    ).preferences.confirm_bake_overwrite


def get_domain_min() -> mathutils.Vector:
    return bpy.context.preferences.addons.get(  # ty:ignore[possibly-missing-attribute]
        __package__  # ty:ignore[invalid-argument-type]
    ).preferences.domain_min


def get_domain_max() -> mathutils.Vector:
    return bpy.context.preferences.addons.get(  # ty:ignore[possibly-missing-attribute]
        __package__  # ty:ignore[invalid-argument-type]
    ).preferences.domain_max


def register_preferences():
    bpy.utils.register_class(SquishyVolumesPreferences)


def unregister_preferences():
    bpy.utils.unregister_class(SquishyVolumesPreferences)
