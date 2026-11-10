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

import bpy  # ty: ignore[unresolved-import]
import mathutils  # ty: ignore[unresolved-import]
import numpy as np  # ty: ignore[unresolved-import]

from . import input_attributes
from .bridge import SimulationInputHandle
from .get_preferences import get_allow_scaled_input, get_domain_max, get_domain_min
from .squishy_volumes_properties import (
    INPUT_TYPE_COLLIDER,
    INPUT_TYPE_PARTICLES,
    PARTICLE_ATTRIBUTE_MAPPING,
    get_input_objects_with_uuid,
)


def create_input_header(sim_props):
    depsgraph = bpy.context.evaluated_depsgraph_get()
    depsgraph.update()

    grid_node_size = sim_props.grid_node_size
    simulation_scale = sim_props.simulation_scale
    frames_per_second = sim_props.frames_per_second
    domain_min = get_domain_min()
    domain_min = [domain_min[0], domain_min[1], domain_min[2]]
    domain_max = get_domain_max()
    domain_max = [domain_max[0], domain_max[1], domain_max[2]]

    consts = {
        # TODO: add to preferences?
        "leaf_size": grid_node_size * 2.0,
        "leaf_threshold": 16,
        "grid_node_size": grid_node_size,
        "simulation_scale": simulation_scale,
        "frames_per_second": frames_per_second,
        "domain_min": domain_min,
        "domain_max": domain_max,
    }

    objects = {}

    collider_inputs = 0
    for input_obj in get_input_objects_with_uuid(sim_props.uuid):
        evaluated_obj = input_obj.evaluated_get(depsgraph)

        mesh = evaluated_obj.evaluated_get(depsgraph).data
        name = evaluated_obj.name
        ty = evaluated_obj.squishy_volumes.input_type
        if ty == INPUT_TYPE_PARTICLES:
            objects[name] = {
                INPUT_TYPE_PARTICLES: {
                    "num_particles": len(mesh.vertices),
                    "alembic_attributes": ["Position"]
                    + [
                        rust_identifiers
                        for alembic_attribute in evaluated_obj.squishy_volumes.alembic_attributes
                        if alembic_attribute.enabled
                        for rust_identifiers in PARTICLE_ATTRIBUTE_MAPPING[
                            alembic_attribute.name
                        ]
                    ],
                }
            }
        if ty == INPUT_TYPE_COLLIDER:
            objects[name] = {
                INPUT_TYPE_COLLIDER: {
                    "collider_id": collider_inputs,
                    "num_vertices": len(mesh.vertices),
                    "num_triangles": len(mesh.loop_triangles),
                }
            }
            collider_inputs += 1

    if collider_inputs > 16:
        raise RuntimeError(f"""More than 16 colliders (you have {collider_inputs})

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

    attribute.data.foreach_get(info.attr, array)

    return array


def triangles_to_numpy_array(
    *,
    mesh: bpy.types.Mesh,
) -> np.ndarray:
    n = len(mesh.loop_triangles) * 3
    array = np.empty(n, dtype="int32")
    mesh.loop_triangles.foreach_get("vertices", array)

    return array


def _bulk_meta(obj, input_type: str, key: str) -> dict:
    return {"object_name": obj.name, "captured_attribute": {input_type: key}}


def _record_bulk(*, sim_input_handle: SimulationInputHandle, meta: dict, bulk):
    if bulk.dtype == "bool":
        sim_input_handle.record_input_bool(meta=meta, bulk=bulk)
    elif bulk.dtype == "float32":
        sim_input_handle.record_input_float(meta=meta, bulk=bulk)
    elif bulk.dtype == "int32":
        sim_input_handle.record_input_int(meta=meta, bulk=bulk)
    else:
        raise RuntimeError(f"{bulk.dtype} input bulk not handled yet")


def _record_attributes(
    *,
    sim_input_handle: SimulationInputHandle,
    obj,
    input_type: str,
    attributes: tuple[input_attributes.InputAttribute, ...],
):
    mesh = obj.data
    for attribute in attributes:
        if attribute.name not in mesh.attributes:
            continue
        _record_bulk(
            sim_input_handle=sim_input_handle,
            meta=_bulk_meta(obj, input_type, attribute.key),
            bulk=attribute_to_numpy_array(
                mesh=mesh, attribute=mesh.attributes[attribute.name]
            ),
        )


def capture_input_frame(
    *,
    sim_props,
    sim_input_handle: SimulationInputHandle,
) -> bool:
    animated_globals = {
        "gravity_x": sim_props.gravity[0],
        "gravity_y": sim_props.gravity[1],
        "gravity_z": sim_props.gravity[2],
        "goal_stiffness": sim_props.goal_stiffness,
        "goal_damping": sim_props.goal_damping,
        "damping": sim_props.damping,
    }
    frame_start = {"animated_globals": animated_globals}

    sim_input_handle.start_frame(frame_start=frame_start)

    depsgraph = bpy.context.evaluated_depsgraph_get()
    depsgraph.update()

    for input_obj in get_input_objects_with_uuid(sim_props.uuid):
        evaluated_obj = input_obj.evaluated_get(depsgraph)
        mesh = evaluated_obj.data
        input_type = evaluated_obj.squishy_volumes.input_type

        if input_type == INPUT_TYPE_PARTICLES:
            if not get_allow_scaled_input() and evaluated_obj.scale != mathutils.Vector(
                (1.0, 1.0, 1.0)
            ):
                raise RuntimeError(
                    f"""{evaluated_obj.name} is scaled to {evaluated_obj.scale}.
Your material will be initially compressed or stretched!

If that is what you want, enable the addon preference
"Allow Scaled Input".

Or apply the object scale."""
                )

            _record_attributes(
                sim_input_handle=sim_input_handle,
                obj=evaluated_obj,
                input_type=input_type,
                attributes=input_attributes.PARTICLE_ATTRIBUTES,
            )

        if input_type == INPUT_TYPE_COLLIDER:
            assert len(mesh.polygons) == len(mesh.loop_triangles), (
                "Is the mesh triangulated?"
            )

            # the topology is recorded only once and can't change (yet?)
            if sim_props.capture_start_frame == bpy.context.scene.frame_current:
                _record_bulk(
                    sim_input_handle=sim_input_handle,
                    meta=_bulk_meta(evaluated_obj, input_type, "Triangles"),
                    bulk=triangles_to_numpy_array(mesh=mesh),
                )

            _record_attributes(
                sim_input_handle=sim_input_handle,
                obj=evaluated_obj,
                input_type=input_type,
                attributes=input_attributes.COLLIDER_ATTRIBUTES,
            )

    return True
