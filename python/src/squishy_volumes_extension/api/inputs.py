# SPDX-License-Identifier: AGPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import math

import bpy

from ..assets import (
    create_geometry_nodes_generate_collider,
    create_geometry_nodes_generate_particles,
    create_geometry_nodes_visualize_particles,
)
from ..errors import InvalidInput
from ..squishy_volumes_properties import (
    INPUT_TYPE_COLLIDER,
    INPUT_TYPE_PARTICLES,
    PARTICLE_ATTRIBUTE_MAPPING,
    get_simulation_objects,
)
from ..util import copy_simple_property_group
from .tools import add_modifier_drivers

PARTICLES = INPUT_TYPE_PARTICLES
COLLIDER = INPUT_TYPE_COLLIDER
GENERATORS = {
    PARTICLES: create_geometry_nodes_generate_particles,
    COLLIDER: create_geometry_nodes_generate_collider,
}


def input_problem(obj: bpy.types.Object) -> str | None:
    if obj.type != "MESH":
        return f"{obj.name} is not a mesh"
    return None


def is_input(obj: bpy.types.Object) -> bool:
    uuid = obj.squishy_volumes.uuid
    return any(
        sim_obj.squishy_volumes.uuid == uuid
        for sim_obj in get_simulation_objects()
        if uuid and sim_obj != obj
    )


def add_input(
    sim_obj: bpy.types.Object,
    obj: bpy.types.Object,
    input_type: str = PARTICLES,
    *,
    default_generation: bool = True,
    output_object: bool = True,
    default_visualization: bool = True,
) -> bpy.types.Object | None:
    if input_type not in GENERATORS:
        raise InvalidInput(f"Unknown input type {input_type}")
    problem = input_problem(obj)
    if problem is not None:
        raise InvalidInput(problem)

    input_props = obj.squishy_volumes
    input_props.input_type = input_type
    input_props.add_default_generation = default_generation
    input_props.add_output_object = output_object
    input_props.add_default_visualization = default_visualization
    input_props.uuid = sim_obj.squishy_volumes.uuid

    if default_generation:
        _add_default_generation(sim_obj, obj)
    if input_type != PARTICLES:
        return None

    reset_alembic_attributes(obj)
    if not output_object:
        return None
    output_obj = _add_output_object(sim_obj, obj)
    if default_visualization:
        modifier = output_obj.modifiers.new("Squishy Volumes Output", type="NODES")
        modifier.node_group = create_geometry_nodes_visualize_particles()
    return output_obj


def remove_input(obj: bpy.types.Object):
    obj.squishy_volumes.uuid = ""


def reset_alembic_attributes(obj: bpy.types.Object):
    alembic_attributes = obj.squishy_volumes.alembic_attributes
    alembic_attributes.clear()
    for attribute_name in PARTICLE_ATTRIBUTE_MAPPING:
        alembic_attributes.add().name = attribute_name


def copy_alembic_attributes(
    source: bpy.types.Object, targets: list[bpy.types.Object]
) -> list[bpy.types.Object]:
    input_type = source.squishy_volumes.input_type
    updated = [
        target
        for target in targets
        if target != source and target.squishy_volumes.input_type == input_type
    ]
    for target in updated:
        target_attributes = target.squishy_volumes.alembic_attributes
        target_attributes.clear()
        for attribute in source.squishy_volumes.alembic_attributes:
            copy_simple_property_group(source=attribute, target=target_attributes.add())
    return updated


def _add_default_generation(sim_obj: bpy.types.Object, obj: bpy.types.Object):
    modifier = obj.modifiers.new("Squishy Volumes Input", type="NODES")
    modifier.node_group = GENERATORS[obj.squishy_volumes.input_type]()
    add_modifier_drivers(sim_obj, modifier)


def _add_output_object(
    sim_obj: bpy.types.Object, obj: bpy.types.Object
) -> bpy.types.Object:
    output_name = f"{obj.name} - Output"
    output_obj = bpy.data.objects.new(
        name=output_name, object_data=bpy.data.pointclouds.new(output_name)
    )
    output_obj.rotation_euler.x = math.radians(-90)
    for collection in obj.users_collection:
        collection.objects.link(output_obj)
    modifier = output_obj.modifiers.new(
        "Squishy Volumes Load Output", "MESH_SEQUENCE_CACHE"
    )
    modifier.cache_file = sim_obj.squishy_volumes.cache_file
    modifier.object_path = f"/{obj.name}"
    return output_obj
