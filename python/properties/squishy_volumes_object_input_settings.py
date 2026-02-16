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

INPUT_TYPE_PARTICLES = "Particles"
INPUT_TYPE_COLLIDER = "Collider"


class Squishy_Volumes_Object_Input_Settings(bpy.types.PropertyGroup):
    input_type: bpy.props.EnumProperty(
        items=[
            (INPUT_TYPE_PARTICLES,) * 3,
            (INPUT_TYPE_COLLIDER,) * 3,
        ],  # ty:ignore[invalid-argument-type]
        name="Type",
        description="""TODO""",
        default=INPUT_TYPE_PARTICLES,
        options=set(),
    )  # type: ignore
