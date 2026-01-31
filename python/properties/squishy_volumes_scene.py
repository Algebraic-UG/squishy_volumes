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

from ..properties.util import get_selected_input_object

from .squishy_volumes_simulation import Squishy_Volumes_Simulation


def selectable_simulations(_, context):
    return [
        (simulation.uuid, simulation.name, "")
        for simulation in context.scene.squishy_volumes_scene.simulations
    ]


def update_object_selection(_, context):
    obj = get_selected_input_object(context)
    if obj is not None:
        if obj.name in context.view_layer.objects:
            context.view_layer.objects.active = obj
            obj.select_set(True)


class Squishy_Volumes_Scene(bpy.types.PropertyGroup):
    simulations: bpy.props.CollectionProperty(
        type=Squishy_Volumes_Simulation,
        name="Simulations",
        description="Squishy Volumes Simluations that can receive inputs and produce outputs.",
        options=set(),
    )  # type: ignore
    selected_simulation: bpy.props.EnumProperty(
        items=selectable_simulations,
        name="Selected Simulation",
        description="Most operations assume this simulation as the target.",
        options=set(),
    )  # type: ignore
    selected_input_object: bpy.props.IntProperty(
        name="Selected Input Object",
        description="The selected input object can have it's setting edited or removed.",
        update=update_object_selection,
        options=set(),
    )  # type: ignore
    selected_output_object: bpy.props.IntProperty(
        name="Selected Output Object",
        description="The selected output object can have it's setting edited or removed.",
        update=update_object_selection,
        options=set(),
    )  # type: ignore
