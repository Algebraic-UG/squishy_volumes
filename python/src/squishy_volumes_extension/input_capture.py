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

import numpy as np
from .bridge import SimulationInputHandle
from .squishy_volumes_properties import (
    Squishy_Volumes_Properties_Simulation,
    get_input_objects_with_uuid,
    get_input_objects,
    INPUT_TYPE_PARTICLES,
    INPUT_TYPE_COLLIDER,
)
from .preferences import get_domain_min, get_domain_max, get_max_num_particles


def create_input_header(sim_props):
    scene = bpy.context.scene
    depsgraph = bpy.context.evaluated_depsgraph_get()

    grid_node_size = sim_props.grid_node_size
    simulation_scale = sim_props.simulation_scale
    frames_per_second = sim_props.frames_per_second
    domain_min = get_domain_min()
    domain_min = [domain_min[0], domain_min[1], domain_min[2]]
    domain_max = get_domain_max()
    domain_max = [domain_max[0], domain_max[1], domain_max[2]]

    max_num_particles = get_max_num_particles()

    consts = {
        # TODO: add to preferences?
        "leaf_size": grid_node_size * 2.0,
        "leaf_threshold": 16,
        #
        "grid_node_size": grid_node_size,
        "simulation_scale": simulation_scale,
        "frames_per_second": frames_per_second,
        "domain_min": domain_min,
        "domain_max": domain_max,
        "max_num_particles": max_num_particles,
    }

    objects = {}

    collider_inputs = 0
    for input_obj in get_input_objects_with_uuid(sim_props.uuid):
        mesh = input_obj.evaluated_get(depsgraph).data
        name = input_obj.name
        ty = input_obj.squishy_volumes.input_type  # ty:ignore[unresolved-attribute]
        if ty == INPUT_TYPE_PARTICLES:
            objects[name] = {
                INPUT_TYPE_PARTICLES: {"num_particles": len(mesh.vertices)}  # ty:ignore[possibly-missing-attribute]
            }
        if ty == INPUT_TYPE_COLLIDER:
            collider_inputs += 1
            objects[name] = {
                INPUT_TYPE_COLLIDER: {
                    "num_vertices": len(mesh.vertices),  # ty:ignore[possibly-missing-attribute]
                    "num_triangles": len(mesh.loop_triangles),  # ty:ignore[possibly-missing-attribute]
                }
            }

    if collider_inputs > 16:
        raise RuntimeError(f"""

More than 16 colliders (you have {collider_inputs})

Please consider joining some of them in a single object,
or reduce the number of collider objects in other ways.

A current technical limitation prevents more separate objects,
let us know if you need more.""")

    return {
        "consts": consts,
        "objects": objects,
    }


class AttributeInfo:
    def __init__(self, *, per_count: int, attr: str, dtype: str):
        self.per_count = per_count
        self.attr = attr
        self.dtype = dtype


ATTRIBUTE_MAP = {
    "FLOAT": AttributeInfo(per_count=1, attr="value", dtype="float32"),
    "INT": AttributeInfo(per_count=1, attr="value", dtype="int32"),
    "BOOLEAN": AttributeInfo(per_count=1, attr="value", dtype="bool"),
    "FLOAT_VECTOR": AttributeInfo(per_count=3, attr="vector", dtype="float32"),
    "FLOAT_COLOR": AttributeInfo(per_count=4, attr="color", dtype="float32"),
    "QUATERNION": AttributeInfo(per_count=4, attr="value", dtype="float32"),
    "FLOAT4X4": AttributeInfo(per_count=16, attr="value", dtype="float32"),
}


def attribute_to_numpy_array(
    *,
    mesh: bpy.types.Mesh,
    attribute: bpy.types.Attribute,
) -> np.ndarray:
    info = ATTRIBUTE_MAP[attribute.data_type]

    if attribute.domain == "POINT":
        n = len(mesh.vertices) * info.per_count
    elif attribute.domain == "FACE":
        n = len(mesh.polygons) * info.per_count
    else:
        raise RuntimeError("Unsupported Attribute Domain")

    array = np.empty(n, dtype=info.dtype)

    attribute.data.foreach_get(info.attr, array)  # ty:ignore[unresolved-attribute]

    return array


def triangles_to_numpy_array(
    *,
    mesh: bpy.types.Mesh,
) -> np.ndarray:
    n = len(mesh.loop_triangles) * 3
    array = np.empty(n, dtype="int32")
    mesh.loop_triangles.foreach_get("vertices", array)

    return array


def capture_input_frame(
    *,
    sim_props,
    sim_input_handle: SimulationInputHandle,
):
    gravity = [
        sim_props.gravity[0],
        sim_props.gravity[1],
        sim_props.gravity[2],
    ]
    frame_start = {"gravity": gravity}

    sim_input_handle.start_frame(frame_start=frame_start)

    depsgraph = bpy.context.evaluated_depsgraph_get()

    for input_obj in get_input_objects_with_uuid(sim_props.uuid):
        mesh = input_obj.evaluated_get(depsgraph).data
        attributes = mesh.attributes  # ty:ignore[possibly-missing-attribute]
        input_type = input_obj.squishy_volumes.input_type  # ty:ignore[unresolved-attribute]

        def record(
            *, python_name: str | None, rust_name: str, triangle_indices: bool = False
        ):
            meta = {
                "object_name": input_obj.name,
                "captured_attribute": {input_type: rust_name},
            }
            if triangle_indices:
                bulk = triangles_to_numpy_array(mesh=mesh)  # ty:ignore[invalid-argument-type]
            else:
                if python_name not in attributes:  # ty:ignore[unsupported-operator]
                    return
                bulk = attribute_to_numpy_array(
                    mesh=mesh,  # ty:ignore[invalid-argument-type]
                    attribute=attributes[python_name],
                )
            if bulk.dtype == "bool":
                sim_input_handle.record_input_bool(meta=meta, bulk=bulk)
            elif bulk.dtype == "float32":
                sim_input_handle.record_input_float(meta=meta, bulk=bulk)
            elif bulk.dtype == "int32":
                sim_input_handle.record_input_int(meta=meta, bulk=bulk)
            else:
                raise RuntimeError(f"{bulk.dtype} input bulk not handled yet")

        if input_type == INPUT_TYPE_PARTICLES:
            record(python_name="squishy_volumes_is_solid", rust_name="IsSolid")
            record(python_name="squishy_volumes_is_fluid", rust_name="IsFluid")
            record(
                python_name="squishy_volumes_use_viscosity", rust_name="UseViscosity"
            )
            record(
                python_name="squishy_volumes_use_sand_alpha", rust_name="UseSandAlpha"
            )
            record(python_name="squishy_volumes_has_goal", rust_name="HasGoal")
            record(python_name="squishy_volumes_transform", rust_name="Transforms")
            record(python_name="squishy_volumes_size", rust_name="Sizes")
            record(python_name="squishy_volumes_density", rust_name="Densities")
            record(
                python_name="squishy_volumes_youngs_modulus",
                rust_name="YoungsModuluses",
            )
            record(
                python_name="squishy_volumes_poissons_ratio", rust_name="PoissonsRatios"
            )
            record(
                python_name="squishy_volumes_initial_position",
                rust_name="InitialPositions",
            )
            record(python_name="squishy_volumes_velocity", rust_name="InitialVelocity")
            record(
                python_name="squishy_volumes_viscosity_dynamic",
                rust_name="ViscosityDynamic",
            )
            record(
                python_name="squishy_volumes_viscosity_bulk", rust_name="ViscosityBulk"
            )
            record(python_name="squishy_volumes_exponent", rust_name="Exponent")
            record(python_name="squishy_volumes_bulk_modulus", rust_name="BulkModulus")
            record(python_name="squishy_volumes_sand_alpha", rust_name="SandAlpha")
            record(
                python_name="squishy_volumes_goal_position", rust_name="GoalPositions"
            )

        if input_type == INPUT_TYPE_COLLIDER:
            assert len(mesh.polygons) == len(mesh.loop_triangles), (  # ty:ignore[possibly-missing-attribute]
                "Is the mesh triangulated?"
            )

            record(python_name="squishy_volumes_position", rust_name="VertexPositions")
            record(python_name=None, rust_name="Triangles", triangle_indices=True)
            record(
                python_name="squishy_volumes_friction", rust_name="TriangleFrictions"
            )

    sim_input_handle.finish_frame()
