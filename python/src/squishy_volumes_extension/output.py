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

import json
import mathutils
import numpy as np

from .magic_consts import (
    SQUISHY_VOLUMES_DISTANCE,
    SQUISHY_VOLUMES_ELASTIC_ENERGY,
    SQUISHY_VOLUMES_INITIAL_POSITION,
    SQUISHY_VOLUMES_MASS,
    SQUISHY_VOLUMES_NORMAL,
    SQUISHY_VOLUMES_PRESSURE,
    SQUISHY_VOLUMES_FLAGS,
    SQUISHY_VOLUMES_TRANSFORM,
    SQUISHY_VOLUMES_VELOCITY,
    PARTICLES,
    GRID,
    SQUISHY_VOLUMES_SIZE,
    SQUISHY_VOLUMES_COLLIDER_BITS,
)

from .nodes import (
    create_geometry_nodes_surface_samples,
    create_geometry_nodes_particles,
    create_material_display_uvw,
    create_geometry_nodes_grid,
)
from .nodes.drivers import add_drivers
from .util import (
    fill_mesh_with_positions,
    fill_mesh_with_vertices_and_triangles,
    fix_quaternion_order,
)
from .bridge import Simulation


def create_default_visualization(obj, uuid):
    output_type = obj.squishy_volumes_object.output_settings.output_type
    modifier = obj.modifiers.new("Squishy Volumes Default Visualization", type="NODES")

    if output_type == GRID:
        modifier.node_group = create_geometry_nodes_grid()
    if output_type == PARTICLES:
        modifier.node_group = create_geometry_nodes_particles()
        modifier["Socket_9"] = create_material_display_uvw()
        modifier["Socket_12"] = bpy.data.objects.get(
            obj.squishy_volumes_object.output_settings.input_name
        )
    add_drivers(uuid, modifier)


def add_attribute(mesh, array, attribute_name, attribute_type, domain="POINT"):
    attribute = mesh.attributes.get(attribute_name)
    if attribute is None:
        attribute = mesh.attributes.new(
            name=attribute_name, type=attribute_type, domain=domain
        )
    if attribute_type == "FLOAT_VECTOR":
        attribute.data.foreach_set("vector", array)
    elif attribute_type == "FLOAT_COLOR":
        attribute.data.foreach_set("color", array)
    else:
        attribute.data.foreach_set("value", array)


def sync_output(sim: Simulation, obj: bpy.types.Object, num_colliders: int, frame: int):
    output_settings = obj.squishy_volumes_object.output_settings  # ty:ignore[unresolved-attribute]

    if output_settings.output_type == GRID:
        ffa_f32 = lambda attribute: sim.fetch_flat_attribute_f32(
            frame=frame,
            attribute={"Grid": attribute},
        )
        ffa_i32 = lambda attribute: sim.fetch_flat_attribute_i32(
            frame=frame,
            attribute={"Grid": attribute},
        )

        fill_mesh_with_positions(obj.data, ffa_f32("Positions"))
        if output_settings.grid_collider_bits:
            add_attribute(
                obj.data,
                ffa_i32("ColliderBits"),
                SQUISHY_VOLUMES_COLLIDER_BITS,
                "INT",
            )

        if output_settings.grid_masses:
            add_attribute(
                obj.data,
                ffa_f32("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if output_settings.grid_velocities:
            add_attribute(
                obj.data,
                ffa_f32("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )

    if output_settings.output_type == PARTICLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa_f32 = lambda attribute: sim.fetch_flat_attribute_f32(
            frame=frame,
            attribute={
                "Object": {
                    "name": output_settings.input_name,
                    "attribute": attribute,
                }
            },
        )
        ffa_i32 = lambda attribute: sim.fetch_flat_attribute_i32(
            frame=frame,
            attribute={
                "Object": {
                    "name": output_settings.input_name,
                    "attribute": attribute,
                }
            },
        )

        fill_mesh_with_positions(obj.data, ffa_f32("Positions"))
        if output_settings.particle_flags:
            add_attribute(
                obj.data,
                ffa_i32("States"),
                SQUISHY_VOLUMES_FLAGS,
                "INT",
            )
        if output_settings.particle_masses:
            add_attribute(
                obj.data,
                ffa_f32("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if output_settings.particle_initial_volumes:
            add_attribute(
                obj.data,
                ffa_f32("InitialVolumes"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_settings.particle_initial_positions:
            add_attribute(
                obj.data,
                ffa_f32("InitialPositions"),
                SQUISHY_VOLUMES_INITIAL_POSITION,
                "FLOAT_VECTOR",
            )
        if output_settings.particle_velocities:
            add_attribute(
                obj.data,
                ffa_f32("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
        if output_settings.particle_sizes:
            add_attribute(
                obj.data,
                ffa_f32("Sizes"),
                SQUISHY_VOLUMES_SIZE,
                "FLOAT",
            )
        if output_settings.particle_transformations:
            add_attribute(
                obj.data,
                ffa_f32("Transformations"),
                SQUISHY_VOLUMES_TRANSFORM,
                "FLOAT4X4",
            )
        if output_settings.particle_energies:
            add_attribute(
                obj.data,
                ffa_f32("ElasticEnergies"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_settings.particle_collider_bits:
            add_attribute(
                obj.data,
                ffa_i32("ColliderBits"),
                SQUISHY_VOLUMES_COLLIDER_BITS,
                "INT",
            )
