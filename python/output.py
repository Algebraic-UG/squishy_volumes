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
    COLLIDER_MESH,
    COLLIDER_SAMPLES,
    FLUID_PARTICLES,
    GRID_COLLIDER_DISTANCE,
    GRID_MOMENTUM_CONFORMED,
    GRID_MOMENTUM_FREE,
    INPUT_MESH,
    SOLID_PARTICLES,
)
from .nodes.drivers import add_drivers
from .nodes.geometry_nodes_grid_momentum import create_geometry_nodes_grid_momentum
from .nodes.geometry_nodes_surface_samples import create_geometry_nodes_surface_samples
from .nodes.geometry_nodes_grid_distance import create_geometry_nodes_grid_distance
from .nodes.geometry_nodes_particles import create_geometry_nodes_particles
from .nodes.material_display_uvw import create_material_display_uvw
from .util import (
    fill_mesh_with_positions,
    fill_mesh_with_vertices_and_triangles,
    fix_quaternion_order,
)
from .bridge import fetch_flat_attribute


def create_output(simulation, obj, frame):
    mpm = obj.squishy_volumes_object
    if mpm.output_type == COLLIDER_MESH or mpm.output_type == INPUT_MESH:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps({"Mesh": {"name": mpm.input_name, "attribute": attribute}}),
        )

        fill_mesh_with_vertices_and_triangles(
            obj.data, ffa("Vertices"), ffa("Triangles")
        )
        obj.location = ffa("Position")
        obj.rotation_mode = "QUATERNION"
        obj.rotation_quaternion = fix_quaternion_order(ffa("Orientation"))
        return

    if mpm.output_type == FLUID_PARTICLES:
        return  # TODO create

    modifier = obj.modifiers.new("Squishy Volumes Default", type="NODES")

    if mpm.output_type == GRID_COLLIDER_DISTANCE:
        modifier.node_group = create_geometry_nodes_grid_distance()
    if mpm.output_type == GRID_MOMENTUM_FREE:
        modifier.node_group = create_geometry_nodes_grid_momentum()
    if mpm.output_type == GRID_MOMENTUM_CONFORMED:
        modifier.node_group = create_geometry_nodes_grid_momentum()
    if mpm.output_type == SOLID_PARTICLES:
        modifier.node_group = create_geometry_nodes_particles()
        modifier["Socket_10"] = create_material_display_uvw()
    if mpm.output_type == COLLIDER_SAMPLES:
        modifier.node_group = create_geometry_nodes_surface_samples()

    add_drivers(simulation, modifier)


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


def sync_output(simulation, obj, num_colliders, frame):
    mpm = obj.squishy_volumes_object

    if mpm.sync_once and frame != mpm.sync_once_frame:
        return

    if mpm.output_type == GRID_COLLIDER_DISTANCE:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps({"GridColliderDistance": attribute}),
        )

        fill_mesh_with_positions(obj.data, ffa("Positions"))
        if mpm.optional_attributes.grid_collider_distances:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderDistances": collider_idx}),
                    f"{SQUISHY_VOLUMES_DISTANCE}_{collider_idx}",
                    "FLOAT",
                )
        if mpm.optional_attributes.grid_collider_normals:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderDistanceNormals": collider_idx}),
                    f"{SQUISHY_VOLUMES_NORMAL}_{collider_idx}",
                    "FLOAT_VECTOR",
                )

    if mpm.output_type == GRID_MOMENTUM_FREE:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps({"GridMomentums": {"Free": attribute}}),
        )

        fill_mesh_with_positions(obj.data, ffa("Positions"))
        if mpm.optional_attributes.grid_momentum_masses:
            add_attribute(
                obj.data,
                ffa("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if mpm.optional_attributes.grid_momentum_velocities:
            add_attribute(
                obj.data,
                ffa("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
    if mpm.output_type == GRID_MOMENTUM_CONFORMED:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps(
                {
                    "GridMomentums": {
                        "Conformed": {"name": mpm.input_name, "attribute": attribute}
                    }
                }
            ),
        )

        fill_mesh_with_positions(obj.data, ffa("Positions"))
        if mpm.optional_attributes.grid_momentum_masses:
            add_attribute(
                obj.data,
                ffa("Masses"),
                SQUISHY_VOLUMES_MASS,
                "FLOAT",
            )
        if mpm.optional_attributes.grid_momentum_velocities:
            add_attribute(
                obj.data,
                ffa("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )

    if mpm.output_type == SOLID_PARTICLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps(
                {
                    "Object": {
                        "name": mpm.input_name,
                        "attribute": {"Solid": attribute},
                    }
                }
            ),
        )

        fill_mesh_with_positions(obj.data, ffa("Positions"))

        if mpm.optional_attributes.solid_states:
            add_attribute(
                obj.data,
                ffa("States"),
                SQUISHY_VOLUMES_STATE,
                "FLOAT",
            )
        if mpm.optional_attributes.solid_masses:
            add_attribute(
                obj.data,
                ffa("Masses"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if mpm.optional_attributes.solid_initial_volumes:
            add_attribute(
                obj.data,
                ffa("InitialVolumes"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if mpm.optional_attributes.solid_initial_positions:
            add_attribute(
                obj.data,
                ffa("InitialPositions"),
                SQUISHY_VOLUMES_INITIAL_POSITION,
                "FLOAT_VECTOR",
            )
        if mpm.optional_attributes.solid_velocities:
            add_attribute(
                obj.data,
                ffa("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
        if mpm.optional_attributes.solid_transformations:
            add_attribute(
                obj.data,
                ffa("Transformations"),
                SQUISHY_VOLUMES_TRANSFORM,
                "FLOAT4X4",
            )
        if mpm.optional_attributes.solid_energies:
            add_attribute(
                obj.data,
                ffa("ElasticEnergies"),
                SQUISHY_VOLUMES_ELASTIC_ENERGY,
                "FLOAT",
            )
        if mpm.optional_attributes.solid_collider_insides:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderInsides": collider_idx}),
                    f"{SQUISHY_VOLUMES_COLLIDER_INSIDE}_{collider_idx}",
                    "FLOAT",
                )
    if mpm.output_type == FLUID_PARTICLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps(
                {"Object": {"name": mpm.input_name, "attribute": {"Fluid": attribute}}}
            ),
        )
        fill_mesh_with_positions(obj.data, ffa("Positions"))

        if mpm.optional_attributes.fluid_states:
            add_attribute(
                obj.data,
                ffa("States"),
                SQUISHY_VOLUMES_STATE,
                "FLOAT",
            )
        if mpm.optional_attributes.fluid_initial_positions:
            add_attribute(
                obj.data,
                ffa("InitialPositions"),
                SQUISHY_VOLUMES_INITIAL_POSITION,
                "FLOAT_VECTOR",
            )
        if mpm.optional_attributes.fluid_velocities:
            add_attribute(
                obj.data,
                ffa("Velocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
        if mpm.optional_attributes.fluid_transformations:
            add_attribute(
                obj.data,
                ffa("Transformations"),
                SQUISHY_VOLUMES_TRANSFORM,
                "FLOAT4X4",
            )
        if mpm.optional_attributes.fluid_collider_insides:
            for collider_idx in range(0, num_colliders):
                add_attribute(
                    obj.data,
                    ffa({"ColliderInsides": collider_idx}),
                    f"{SQUISHY_VOLUMES_COLLIDER_INSIDE}_{collider_idx}",
                    "FLOAT",
                )
        if mpm.optional_attributes.fluid_pressures:
            add_attribute(
                obj.data,
                ffa("Pressures"),
                SQUISHY_VOLUMES_PRESSURE,
                "FLOAT",
            )

    if mpm.output_type == COLLIDER_SAMPLES:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps(
                {
                    "Object": {
                        "name": mpm.input_name,
                        "attribute": {"Collider": attribute},
                    }
                }
            ),
        )

        fill_mesh_with_positions(obj.data, ffa("Samples"))

        if mpm.optional_attributes.collider_normals:
            add_attribute(
                obj.data,
                ffa("SampleNormals"),
                SQUISHY_VOLUMES_NORMAL,
                "FLOAT_VECTOR",
            )
        if mpm.optional_attributes.collider_velocities:
            add_attribute(
                obj.data,
                ffa("SampleVelocities"),
                SQUISHY_VOLUMES_VELOCITY,
                "FLOAT_VECTOR",
            )
    if mpm.output_type == COLLIDER_MESH:
        # pylint: disable=unnecessary-lambda-assignment
        ffa = lambda attribute: fetch_flat_attribute(
            simulation,
            frame,
            json.dumps(
                {
                    "Object": {
                        "name": mpm.input_name,
                        "attribute": {"Collider": attribute},
                    }
                }
            ),
        )
        obj.matrix_world = mathutils.Matrix(np.reshape(ffa("Transformation"), (4, 4)))
        obj.matrix_world.transpose()

    if mpm.output_type == INPUT_MESH:
        pass
