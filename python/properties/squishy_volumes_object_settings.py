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

from ..properties.util import get_simulation_specific_settings, is_some_and

# these have to match the enum in core::input::InputObjectType
OBJECT_ENUM_PARTICLES = "Particles"


def get_input_objects_type(simulation, input_type):
    return [
        obj
        for obj in bpy.data.objects
        if is_some_and(
            get_simulation_specific_settings(simulation, obj),
            lambda settings: settings.object_enum == input_type,
        )
    ]


def get_input_particle_objects(simulation):
    return get_input_objects_type(simulation, OBJECT_ENUM_PARTICLES)


class Squishy_Volumes_Object_Settings(bpy.types.PropertyGroup):
    simulation_uuid: bpy.props.StringProperty(
        name="Simulation UUID",
        description="Backreference to the simulation these settings are meant for.",
        default="unassigned",
        options=set(),
    )  # type: ignore

    object_enum: bpy.props.EnumProperty(
        items=[
            (OBJECT_ENUM_PARTICLES,) * 3,
        ],  # ty:ignore[invalid-argument-type]
        name="Type",
        description="""TODO""",
        default=OBJECT_ENUM_PARTICLES,
        options=set(),
    )  # type: ignore
