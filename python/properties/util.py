# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Blended MPM extension.
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

from ..util import get_simulation_by_uuid


def get_input_objects(simulation):
    return [
        obj
        for obj in bpy.data.objects
        if has_simulation_specific_settings(simulation, obj)
    ]


def get_output_objects(simulation):
    return [
        obj
        for obj in bpy.data.objects
        if obj.blended_mpm_object.simulation_uuid == simulation.uuid
    ]


def get_output_collections(simulation):
    return [
        collection
        for collection in bpy.data.collections
        if collection.blended_mpm_collection.simulation_uuid == simulation.uuid
    ]


def has_simulation_specific_settings(simulation, obj):
    return any(
        [
            settings.simulation_uuid == simulation.uuid
            for settings in obj.blended_mpm_object.simulation_specific_settings
        ]
    )


def get_simulation_specific_settings(simulation, obj):
    return [
        settings
        for settings in obj.blended_mpm_object.simulation_specific_settings
        if settings.simulation_uuid == simulation.uuid
    ][0]


def get_selected_simulation(context):
    simulations = context.scene.blended_mpm_scene.simulations
    if not simulations:
        return None

    if len(simulations) == 1:
        return simulations[0]

    selected_uuid = context.scene.blended_mpm_scene.selected_simulation
    if not selected_uuid:
        return None

    return get_simulation_by_uuid(selected_uuid)


def get_selected_input_object(context):
    simulation = get_selected_simulation(context)
    if simulation is None:
        return None
    selected_input_object = context.scene.blended_mpm_scene.selected_input_object
    if selected_input_object >= len(context.scene.objects):
        return None
    obj = context.scene.objects[selected_input_object]
    if not has_simulation_specific_settings(simulation, obj):
        return None
    return obj


def get_selected_output_object(context):
    selected_output_object = context.scene.blended_mpm_scene.selected_output_object
    if selected_output_object >= len(context.scene.objects):
        return None
    obj = context.scene.objects[selected_output_object]
    if obj.blended_mpm_object.simulation_uuid == "":
        return None
    return obj
