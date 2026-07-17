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

from ..bridge import SimulationHandle
from ..util import simulation_locked, simulation_input_exists

from .object_simulation import *
from .object_input import *
from .object_output import *

TYPE_NONE = "None"
TYPE_INPUT = "Input"
TYPE_OUTPUT = "Output"


def is_some_and(x, f):
    return x is not None and f(x)


def add_fields_from(source_cls, prefix=""):
    def decorator(target_cls):
        source_annotations = getattr(source_cls, "__annotations__", {})
        new_annotations = {f"{prefix}{k}": v for k, v in source_annotations.items()}

        existing_annotations = getattr(target_cls, "__annotations__", {})

        if new_annotations.keys() & existing_annotations.keys():
            raise RuntimeError("Clobbering annotations")

        existing_annotations.update(new_annotations)
        target_cls.__annotations__ = existing_annotations

        for name in new_annotations:
            setattr(target_cls, name, None)

        return target_cls

    return decorator


# only one set of the fields is valid for use, depending on the type
@add_fields_from(Squishy_Volumes_Properties_Simulation)
@add_fields_from(Squishy_Volumes_Properties_Input)
@add_fields_from(Squishy_Volumes_Properties_Output)
class Squishy_Volumes_Properties(bpy.types.PropertyGroup):
    uuid: bpy.props.StringProperty(
        name="UUID",
        description="Reference to the Simulation.",
        default="unassigned",
        options=set(),
    )  # type: ignore

    type: bpy.props.EnumProperty(
        items=[
            (TYPE_NONE,) * 3,
            (TYPE_SIMULATION,) * 3,
            (TYPE_INPUT,) * 3,
            (TYPE_OUTPUT,) * 3,
        ],  # ty:ignore[invalid-argument-type]
        name="Object Type",
        description="""Describes how this object is used by Squishy Volumes.
It might be unreated, a simulation itself or in/output.""",
        default=TYPE_NONE,
        options=set(),
    )  # type: ignore


def get_input_objects() -> list[bpy.types.Object]:
    return [
        obj
        for obj in bpy.data.objects
        if obj.squishy_volumes.type == TYPE_INPUT  # ty:ignore[unresolved-attribute]
    ]


def get_output_objects() -> list[bpy.types.Object]:
    return [
        obj
        for obj in bpy.data.objects
        if obj.squishy_volumes.type == TYPE_OUTPUT  # ty:ignore[unresolved-attribute]
    ]


def get_input_objects_with_uuid(uuid: str) -> list[bpy.types.Object]:
    return [
        obj
        for obj in get_input_objects()
        if obj.squishy_volumes.uuid == uuid  # ty:ignore[unresolved-attribute]
    ]


def get_output_objects_with_uuid(uuid: str) -> list[bpy.types.Object]:
    return [
        obj
        for obj in get_output_objects()
        if obj.squishy_volumes.uuid == uuid  # ty:ignore[unresolved-attribute]
    ]


def locked_simulations() -> list[bpy.types.Object]:
    return [
        obj
        for obj in get_simulation_objects()
        if not SimulationHandle.exists(uuid=obj.squishy_volumes.uuid)  # ty:ignore[unresolved-attribute]
        and simulation_locked(obj.squishy_volumes.directory)  # ty:ignore[unresolved-attribute]
    ]


def unloaded_simulations(context):
    return [
        obj
        for obj in get_simulation_objects()
        if not SimulationHandle.exists(uuid=obj.squishy_volumes.uuid)  # ty:ignore[unresolved-attribute]
        and simulation_input_exists(obj.squishy_volumes.directory)  # ty:ignore[unresolved-attribute]
    ]
