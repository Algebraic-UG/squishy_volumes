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
from .bridge import SimulationHandle
from .squishy_volumes_properties import Squishy_Volumes_Properties_Output


def create_default_visualization(sim_obj, output_obj):
    output_props = output_obj.squishy_volumes
    modifier = output_obj.modifiers.new(
        "Squishy Volumes Default Visualization", type="NODES"
    )

    if output_props.output_type == GRID:
        modifier.node_group = create_geometry_nodes_grid()
    if output_props.output_type == PARTICLES:
        modifier.node_group = create_geometry_nodes_particles()
        modifier["Socket_9"] = create_material_display_uvw()
        modifier["Socket_12"] = bpy.data.objects.get(output_props.input_name)
    add_drivers(sim_obj, modifier)


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


def sync_output(sim_handle: SimulationHandle, output_obj: bpy.types.Object, frame: int):
    output_props: Squishy_Volumes_Properties_Output = output_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

    if output_props.output_type == GRID:
        ffa_f32 = lambda attribute: sim_handle.fetch_flat_attribute_f32(
            frame=frame,
            attribute={"Grid": attribute},
        )
        ffa_i32 = lambda attribute: sim_handle.fetch_flat_attribute_i32(
            frame=frame,
            attribute={"Grid": attribute},
        )

        fill_mesh_with_positions(output_obj.data, ffa_f32("Positions"))
        if output_props.grid_collider_bits:
            add_attribute(
                output_obj.data,
                ffa_i32("ColliderBits"),
                SQUISHY_VOLUMES_COLLIDER_BITS,
                "INT",
            )

        if output_props.grid_masses:
            add_attribute(
                output_obj.data,
                ffa_f32("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if output_props.grid_velocities:
            add_attribute(
                output_obj.data,
                ffa_f32("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )

    if output_props.output_type == PARTICLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa_f32 = lambda attribute: sim_handle.fetch_flat_attribute_f32(
            frame=frame,
            attribute={
                "Object": {
                    "name": output_props.input_name,
                    "attribute": attribute,
                }
            },
        )
        ffa_i32 = lambda attribute: sim_handle.fetch_flat_attribute_i32(
            frame=frame,
            attribute={
                "Object": {
                    "name": output_props.input_name,
                    "attribute": attribute,
                }
            },
        )

        fill_mesh_with_positions(output_obj.data, ffa_f32("Positions"))
        if output_props.particle_flags:
            add_attribute(
                output_obj.data,
                ffa_i32("States"),
                SQUISHY_VOLUMES_FLAGS,
                "INT",
            )
        if output_props.particle_masses:
            add_attribute(
                output_obj.data,
                ffa_f32("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if output_props.particle_initial_volumes:
            add_attribute(
                output_obj.data,
                ffa_f32("InitialVolumes"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_props.particle_initial_positions:
            add_attribute(
                output_obj.data,
                ffa_f32("InitialPositions"),
                SQUISHY_VOLUMES_INITIAL_POSITION,
                "FLOAT_VECTOR",
            )
        if output_props.particle_velocities:
            add_attribute(
                output_obj.data,
                ffa_f32("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
        if output_props.particle_sizes:
            add_attribute(
                output_obj.data,
                ffa_f32("Sizes"),
                SQUISHY_VOLUMES_SIZE,
                "FLOAT",
            )
        if output_props.particle_transformations:
            add_attribute(
                output_obj.data,
                ffa_f32("Transformations"),
                SQUISHY_VOLUMES_TRANSFORM,
                "FLOAT4X4",
            )
        if output_props.particle_energies:
            add_attribute(
                output_obj.data,
                ffa_f32("ElasticEnergies"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_props.particle_collider_bits:
            add_attribute(
                output_obj.data,
                ffa_i32("ColliderBits"),
                SQUISHY_VOLUMES_COLLIDER_BITS,
                "INT",
            )
