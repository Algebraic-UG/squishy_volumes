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

from .squishy_volumes_object_output_settings import (
    Squishy_Volumes_Object_Output_Settings,
)

IO_NONE = "None"
IO_INPUT = "Input"
IO_OUTPUT = "Output"

INPUT_TYPE_PARTICLES = "Particles"


def get_input_objects(simulation):
    return [
        obj
        for obj in bpy.data.objects
        if obj.squishy_volumes_object.io == IO_INPUT  # ty:ignore[unresolved-attribute]
        and obj.squishy_volumes_object.simulation_uuid == simulation.uuid  # ty:ignore[unresolved-attribute]
    ]


def get_output_objects(simulation):
    return [
        obj
        for obj in bpy.data.objects
        if obj.squishy_volumes_object.io == IO_OUTPUT  # ty:ignore[unresolved-attribute]
        and obj.squishy_volumes_object.simulation_uuid == simulation.uuid  # ty:ignore[unresolved-attribute]
    ]


class Squishy_Volumes_Object(bpy.types.PropertyGroup):
    simulation_uuid: bpy.props.StringProperty(
        name="Simulation UUID",
        description="Reference to the Simulation.",
        default="unassigned",
        options=set(),
    )  # type: ignore

    io: bpy.props.EnumProperty(
        items=[
            (IO_NONE,) * 3,
            (IO_INPUT,) * 3,
            (IO_OUTPUT,) * 3,
        ],  # ty:ignore[invalid-argument-type]
        name="I/O",
        description="""TODO""",
        default=IO_NONE,
        options=set(),
    )  # type: ignore

    input_type: bpy.props.EnumProperty(
        items=[
            (INPUT_TYPE_PARTICLES,) * 3,
        ],  # ty:ignore[invalid-argument-type]
        name="Input Type",
        description="""TODO""",
        default=INPUT_TYPE_PARTICLES,
        options=set(),
    )  # type: ignore

    output_settings: bpy.props.PointerProperty(
        type=Squishy_Volumes_Object_Output_Settings,
        name="Optional Attributes",
        description="Further customization of what outputs are synchronized.",
        options=set(),
    )  # type: ignore

    sync_once: bpy.props.BoolProperty(
        name="Sync Once",
        description="Instead of continously synchronizing, load only a specific frame.",
        default=False,
    )  # type: ignore
    sync_once_frame: bpy.props.IntProperty(
        name="Sync Once Frame",
        description="""Simulation frame to synchronize on.

Only used if 'Sync Once' is active.
When the outputs of a simulation are synchronized on a different frame,
this object is left untouched.""",
        default=0,
    )  # type: ignore
