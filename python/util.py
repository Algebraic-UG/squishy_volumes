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

import base64
import os
from pathlib import Path

import numpy as np
import bpy


def remove_marker(marker_name):
    marker = bpy.context.scene.timeline_markers.get(marker_name)
    if marker is not None:
        bpy.context.scene.timeline_markers.remove(marker)


def add_or_update_marker(marker_name, frame):
    # Check if the marker with the given name already exists
    marker = bpy.context.scene.timeline_markers.get(marker_name)

    if marker is None:
        # If it doesn't exist, create a new marker at the specified frame
        marker = bpy.context.scene.timeline_markers.new(name=marker_name, frame=frame)
    else:
        # If it exists, update its frame
        marker.frame = frame


def get_simulation_obj(simulation, name):
    collection_name = f"{simulation.name}"
    object_name = f"{simulation.name} {name}"
    mesh_name = f"{simulation.name} {name} Mesh"

    collection = bpy.data.collections.get(collection_name)
    if collection is None:
        collection = bpy.data.collections.new(collection_name)
        bpy.context.scene.collection.children.link(collection)
        collection.blended_mpm_collection.simulation_uuid = simulation.uuid

    mesh = bpy.data.meshes.get(mesh_name)
    if mesh is None:
        mesh = bpy.data.meshes.new(mesh_name)

    obj = bpy.data.objects.get(object_name)
    if obj is None:
        obj = bpy.data.objects.new(object_name, mesh)
        obj.blended_mpm_object.input_name = name
        obj.blended_mpm_object.simulation_uuid = simulation.uuid

    if obj.name not in collection.all_objects:
        collection.objects.link(obj)

    return obj


def fill_mesh_with_positions(mesh, positions):
    num_floats = positions.size
    num_vertices = num_floats // 3

    mesh.clear_geometry()
    mesh.vertices.add(num_vertices)  # Pre-allocate vertex space
    mesh.vertices.foreach_set("co", positions)  # Set all coordinates in one go


def fill_mesh_with_vertices_and_triangles(mesh, vertices, triangles):
    mesh.clear_geometry()
    mesh.from_pydata(
        np.reshape(vertices, (vertices.size // 3, 3)),
        [],
        np.reshape(triangles.astype("int32"), (triangles.size // 3, 3)),
    )


def array_to_base64(array):
    base64_str = base64.b64encode(array.tobytes()).decode("utf-8")

    return {"dtype": str(array.dtype), "data": base64_str}


def attribute_to_base64(collection, attribute_name, dtype, per_count):
    n = len(collection) * per_count
    array = np.empty(n, dtype=dtype)
    collection.foreach_get(attribute_name, array)

    return array_to_base64(array)


def get_simulation_idx_by_uuid(uuid):
    return [
        idx
        for idx, simulation in enumerate(
            bpy.context.scene.blended_mpm_scene.simulations
        )
        if simulation.uuid == uuid
    ][0]


def get_simulation_by_uuid(uuid):
    for simulation in bpy.context.scene.blended_mpm_scene.simulations:
        if simulation.uuid == uuid:
            return simulation
    raise RuntimeError(f"There is no simulation with UUID {uuid}")


DEBUG_VISUALS = "Blended MPM Debug Visuals"


def force_ui_redraw():
    for area in bpy.context.window.screen.areas:
        if area.type == "VIEW_3D":
            area.tag_redraw()


def simulation_cache_locked(simulation):
    return os.path.exists(Path(simulation.cache_directory) / "lock")


def simulation_cache_exists(simulation):
    return os.path.exists(Path(simulation.cache_directory) / "setup.json")


def fix_quaternion_order(quaternion):
    return [quaternion[3], quaternion[0], quaternion[1], quaternion[2]]


def dialog_info(message):
    bpy.context.window_manager.invoke_confirm(
        lambda self, _: self.layout.label(text=message),
        title="Blended MPM Info",
        icon="INFO",
    )


# https://blenderartists.org/t/duplicating-pointerproperty-propertygroup-and-collectionproperty/1419096/2?
def copy_simple_property_group(source, target):
    if not hasattr(source, "__annotations__"):
        return
    for prop_name in source.__annotations__.keys():
        try:
            setattr(target, prop_name, getattr(source, prop_name))
        except (AttributeError, TypeError):
            pass
