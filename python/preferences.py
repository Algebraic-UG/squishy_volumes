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

    def draw(self, context: bpy.types.Context) -> None:
        self.layout.prop(self, "confirm_bake_overwrite")


def get_confirm_bake_overwrite() -> bool:
    return bpy.context.preferences.addons.get(  # ty:ignore[possibly-missing-attribute]
        __package__  # ty:ignore[invalid-argument-type]
    ).preferences.confirm_bake_overwrite


def register_preferences():
    bpy.utils.register_class(SquishyVolumesPreferences)


def unregister_preferences():
    bpy.utils.unregister_class(SquishyVolumesPreferences)
