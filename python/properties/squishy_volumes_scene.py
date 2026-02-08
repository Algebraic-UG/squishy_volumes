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

from ..util import obj_by_index

from .squishy_volumes_simulation import Squishy_Volumes_Simulation
from .squishy_volumes_object import (
    IO_INPUT,
    IO_OUTPUT,
    get_input_objects,
)


def get_simulation_by_uuid(scene: bpy.types.Scene, uuid: str):
    for simulation in scene.squishy_volumes_scene.simulations:  # ty:ignore[unresolved-attribute]
        if simulation.uuid == uuid:
            return simulation
    raise RuntimeError(f"There is no simulation with UUID {uuid}")


def get_selected_simulation(scene: bpy.types.Scene):
    simulations = scene.squishy_volumes_scene.simulations  # ty:ignore[unresolved-attribute]
    if not simulations:
        return None

    if len(simulations) == 1:
        return simulations[0]

    selected_uuid = scene.squishy_volumes_scene.selected_simulation  # ty:ignore[unresolved-attribute]
    if selected_uuid is None:
        return None

    return get_simulation_by_uuid(scene, selected_uuid)


def _verify_selected_object(obj: bpy.types.Object, scene: bpy.types.Scene):
    simulation = get_selected_simulation(scene)
    if (
        simulation is None
        or obj.squishy_volumes_object.simulation_uuid != simulation.uuid  # ty:ignore[unresolved-attribute]
    ):
        return None
    return obj


def get_selected_input_object(scene: bpy.types.Scene):
    obj = obj_by_index(scene.squishy_volumes_scene.selected_input_object)  # ty:ignore[unresolved-attribute]
    if obj is None or obj.squishy_volumes_object.io != IO_INPUT:
        return None
    return _verify_selected_object(obj, scene)


def get_selected_output_object(scene: bpy.types.Scene):
    obj = obj_by_index(scene.squishy_volumes_scene.selected_output_object)  # ty:ignore[unresolved-attribute]
    if obj is None or obj.squishy_volumes_object.io != IO_OUTPUT:
        return None
    return _verify_selected_object(obj, scene)


def _selectable_simulations(_, context):
    return [
        (simulation.uuid, simulation.name, "")
        for simulation in context.scene.squishy_volumes_scene.simulations
    ]


def _update_selection(index, context):
    obj = obj_by_index(index)
    if obj is None:
        return

    if obj.name in context.view_layer.objects:
        context.view_layer.objects.active = obj
        obj.select_set(True)


def _on_active_change():
    obj = bpy.context.active_object
    if obj is None:
        return
    index = next(
        i for i, other in enumerate(bpy.data.objects) if other.name == obj.name
    )

    scene = bpy.context.scene.squishy_volumes_scene  # ty:ignore[unresolved-attribute]

    if (
        obj.squishy_volumes_object.io == IO_INPUT  # ty:ignore[unresolved-attribute]
        and scene.selected_input_object != index
    ):
        scene.selected_input_object = index

    if (
        obj.squishy_volumes_object.io == IO_OUTPUT  # ty:ignore[unresolved-attribute]
        and scene.selected_output_object != index
    ):
        scene.selected_output_object = index


_owner = object()


def subscribe_to_selection():
    bpy.msgbus.subscribe_rna(
        key=(bpy.types.LayerObjects, "active"),  # ty:ignore[invalid-argument-type]
        owner=_owner,
        args=(),
        notify=_on_active_change,
    )


def unsubscribe_from_selection():
    bpy.msgbus.clear_by_owner(_owner)


subscribe_to_selection()


class Squishy_Volumes_Scene(bpy.types.PropertyGroup):
    simulations: bpy.props.CollectionProperty(
        type=Squishy_Volumes_Simulation,
        name="Simulations",
        description="Squishy Volumes Simluations that can receive inputs and produce outputs.",
        options=set(),
    )  # type: ignore
    selected_simulation: bpy.props.EnumProperty(
        items=_selectable_simulations,
        name="Selected Simulation",
        description="Most operations assume this simulation as the target.",
        options=set(),
    )  # type: ignore
    selected_input_object: bpy.props.IntProperty(
        name="Selected Input Object",
        description="The selected input object can have it's setting edited or removed.",
        update=lambda self, context: _update_selection(
            self.selected_input_object, context
        ),
        options=set(),
    )  # type: ignore
    selected_output_object: bpy.props.IntProperty(
        name="Selected Output Object",
        description="The selected output object can have it's setting edited or removed.",
        update=lambda self, context: _update_selection(
            self.selected_output_object, context
        ),
        options=set(),
    )  # type: ignore
