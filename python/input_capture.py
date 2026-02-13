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
from .bridge import SimulationInput
from .properties.squishy_volumes_simulation import Squishy_Volumes_Simulation
from .properties.squishy_volumes_object import get_input_objects


def create_input_header(simulation):
    scene = bpy.context.scene

    grid_node_size = simulation.grid_node_size
    simulation_scale = simulation.simulation_scale
    frames_per_second = simulation.frames_per_second
    domain_min = [
        simulation.domain_min[0],
        simulation.domain_min[1],
        simulation.domain_min[2],
    ]
    domain_max = [
        simulation.domain_max[0],
        simulation.domain_max[1],
        simulation.domain_max[2],
    ]

    consts = {
        "grid_node_size": grid_node_size,
        "simulation_scale": simulation_scale,
        "frames_per_second": frames_per_second,
        "domain_min": domain_min,
        "domain_max": domain_max,
    }

    objects = []

    for obj in get_input_objects(simulation):
        name = obj.name
        ty = obj.squishy_volumes_object.input_settings.input_type
        objects.append(
            {
                "name": name,
                "ty": ty,
            }
        )

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
    assert attribute.domain == "POINT"

    info = ATTRIBUTE_MAP[attribute.data_type]

    n = len(mesh.vertices) * info.per_count
    array = np.empty(n, dtype=info.dtype)

    attribute.data.foreach_get(info.attr, array)  # ty:ignore[unresolved-attribute]

    return array


def capture_input_frame(
    *,
    simulation: Squishy_Volumes_Simulation,
    simulation_input: SimulationInput,
):
    gravity = [
        simulation.gravity[0],
        simulation.gravity[1],
        simulation.gravity[2],
    ]
    frame_start = {"gravity": gravity}

    simulation_input.start_frame(frame_start=frame_start)

    depsgraph = bpy.context.evaluated_depsgraph_get()

    for obj in get_input_objects(simulation):
        mesh = obj.evaluated_get(depsgraph).data
        attributes = mesh.attributes

        def record(*, python_name: str, rust_name: str):
            meta = {
                "Particles": {
                    "object_name": obj.name,
                    "captured_attribute": rust_name,
                }
            }
            bulk = attribute_to_numpy_array(
                mesh=mesh,
                attribute=attributes[python_name],
            )
            if bulk.dtype == "float32":
                simulation_input.record_input_float(meta=meta, bulk=bulk)
            elif bulk.dtype == "int32":
                simulation_input.record_input_int(meta=meta, bulk=bulk)
            else:
                raise RuntimeError(f"{bulk.dtype} input bulk not handled yet")

        record(python_name="squishy_volumes_flags", rust_name="Flags")
        record(python_name="squishy_volumes_transform", rust_name="Transforms")
        record(python_name="squishy_volumes_size", rust_name="Sizes")
        record(python_name="squishy_volumes_density", rust_name="Densities")
        record(
            python_name="squishy_volumes_youngs_modulus", rust_name="YoungsModuluses"
        )
        record(python_name="squishy_volumes_poissons_ratio", rust_name="PoissonsRatios")
        record(
            python_name="squishy_volumes_initial_position", rust_name="InitialPositions"
        )
        record(python_name="squishy_volumes_velocity", rust_name="InitialVelocity")
        record(
            python_name="squishy_volumes_viscosity_dynamic",
            rust_name="ViscosityDynamic",
        )
        record(python_name="squishy_volumes_viscosity_bulk", rust_name="ViscosityBulk")
        record(python_name="squishy_volumes_exponent", rust_name="Exponent")
        record(python_name="squishy_volumes_bulk_modulus", rust_name="BulkModulus")
        record(python_name="squishy_volumes_sand_alpha", rust_name="SandAlpha")
        record(python_name="squishy_volumes_goal_position", rust_name="GoalPositions")
        record(
            python_name="squishy_volumes_goal_stiffness", rust_name="GoalStiffnesses"
        )

    simulation_input.finish_frame()
