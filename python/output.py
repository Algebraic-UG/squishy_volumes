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
    SQUISHY_VOLUMES_COLLIDER_INSIDE,
    SQUISHY_VOLUMES_DISTANCE,
    SQUISHY_VOLUMES_ELASTIC_ENERGY,
    SQUISHY_VOLUMES_INITIAL_POSITION,
    SQUISHY_VOLUMES_MASS,
    SQUISHY_VOLUMES_NORMAL,
    SQUISHY_VOLUMES_PRESSURE,
    SQUISHY_VOLUMES_STATE,
    SQUISHY_VOLUMES_TRANSFORM,
    SQUISHY_VOLUMES_VELOCITY,
    COLLIDER_SAMPLES,
    PARTICLES,
    GRID_COLLIDER_DISTANCE,
    GRID_MOMENTUM_CONFORMED,
    GRID_MOMENTUM_FREE,
    INPUT_MESH,
)

from .nodes import (
    create_geometry_nodes_surface_samples,
    create_geometry_nodes_grid_distance,
    create_geometry_nodes_grid_momentum,
    create_geometry_nodes_particles,
    create_material_display_uvw,
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
    if output_type == INPUT_MESH:
        return

    modifier = obj.modifiers.new("Squishy Volumes Default Visualization", type="NODES")

    if output_type == GRID_COLLIDER_DISTANCE:
        modifier.node_group = create_geometry_nodes_grid_distance()
    if output_type in [GRID_MOMENTUM_FREE, GRID_MOMENTUM_CONFORMED]:
        modifier.node_group = create_geometry_nodes_grid_momentum()
    if output_type == PARTICLES:
        modifier.node_group = create_geometry_nodes_particles()
        modifier["Socket_10"] = create_material_display_uvw()
    if output_type == COLLIDER_SAMPLES:
        modifier.node_group = create_geometry_nodes_surface_samples()

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

    if output_settings.sync_once and frame != output_settings.sync_once_frame:
        return

    if output_settings.output_type == INPUT_MESH:
        raise RuntimeError("Not implemented yet")

    if output_settings.output_type == GRID_COLLIDER_DISTANCE:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: sim.fetch_flat_attribute(
            frame=frame,
            attribute={"GridColliderDistance": attribute},
        )

        fill_mesh_with_positions(obj.data, ffa("Positions"))
        if output_settings.grid_collider_distances:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderDistances": collider_idx}),
                    f"{SQUISHY_VOLUMES_DISTANCE}_{collider_idx}",
                    "FLOAT",
                )
        if output_settings.grid_collider_normals:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderDistanceNormals": collider_idx}),
                    f"{SQUISHY_VOLUMES_NORMAL}_{collider_idx}",
                    "FLOAT_VECTOR",
                )

    if output_settings.output_type in [GRID_MOMENTUM_FREE, GRID_MOMENTUM_CONFORMED]:
        if output_settings.output_type == GRID_MOMENTUM_FREE:
            # pylint: disable=unnecessary-lambda-assignment
            ffa = lambda attribute: sim.fetch_flat_attribute(
                frame=frame,
                attribute={"GridMomentums": {"Free": attribute}},
            )
        if output_settings.output_type == GRID_MOMENTUM_CONFORMED:
            # pylint: disable=unnecessary-lambda-assignment
            ffa = lambda attribute: sim.fetch_flat_attribute(
                frame=frame,
                attribute={
                    "GridMomentums": {
                        "Conformed": {
                            "name": output_settings.input_name,
                            "attribute": attribute,
                        }
                    }
                },
            )

        fill_mesh_with_positions(obj.data, ffa("Positions"))
        if output_settings.grid_momentum_masses:
            add_attribute(
                obj.data,
                ffa("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if output_settings.grid_momentum_velocities:
            add_attribute(
                obj.data,
                ffa("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )

    if output_settings.output_type == PARTICLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: sim.fetch_flat_attribute(
            frame=frame,
            attribute={
                "Object": {
                    "name": output_settings.input_name,
                    "attribute": {"Particles": attribute},
                }
            },
        )

        fill_mesh_with_positions(obj.data, ffa("Positions"))
        if output_settings.particle_states:
            add_attribute(
                obj.data,
                ffa("States"),
                SQUISHY_VOLUMES_STATE,
                "FLOAT",
            )
        if output_settings.particle_masses:
            add_attribute(
                obj.data,
                ffa("Masses"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_settings.particle_initial_volumes:
            add_attribute(
                obj.data,
                ffa("InitialVolumes"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_settings.particle_initial_positions:
            add_attribute(
                obj.data,
                ffa("InitialPositions"),
                SQUISHY_VOLUMES_INITIAL_POSITION,
                "FLOAT_VECTOR",
            )
        if output_settings.particle_velocities:
            add_attribute(
                obj.data,
                ffa("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
        if output_settings.particle_transformations:
            add_attribute(
                obj.data,
                ffa("Transformations"),
                SQUISHY_VOLUMES_TRANSFORM,
                "FLOAT4X4",
            )
        if output_settings.particle_energies:
            add_attribute(
                obj.data,
                ffa("ElasticEnergies"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if output_settings.particle_collider_insides:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderInsides": collider_idx}),
                    f"{SQUISHY_VOLUMES_COLLIDER_INSIDE}_{collider_idx}",
                    "FLOAT",
                )

    if output_settings.output_type == COLLIDER_SAMPLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: sim.fetch_flat_attribute(
            frame=frame,
            attribute={
                "Object": {
                    "name": output_settings.input_name,
                    "attribute": {"Collider": attribute},
                }
            },
        )

        fill_mesh_with_positions(obj.data, ffa("Samples"))
        if output_settings.collider_normals:
            add_attribute(
                obj.data,
                ffa("SampleNormals"),
                SQUISHY_VOLUMES_NORMAL,
                "FLOAT_VECTOR",
            )
        if output_settings.collider_velocities:
            add_attribute(
                obj.data,
                ffa("SampleVelocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
