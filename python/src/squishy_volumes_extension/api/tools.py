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

import bmesh
import bpy
import mathutils

from ..assets import (
    create_geometry_nodes_generate_goal_positions,
    create_geometry_nodes_restrict_view,
)
from ..errors import InvalidInput
from ..squishy_volumes_properties import get_simulation_objects

DRIVEN_SOCKETS = {
    "Grid Node Size": ("grid_node_size", "squishy_volumes.grid_node_size"),
    "Start Frame": ("start_frame", "squishy_volumes.capture_start_frame"),
}
MODIFIER_PREFIX = "Squishy Volumes"


def add_goals(
    obj: bpy.types.Object, *, location: mathutils.Vector | None = None
) -> tuple[bpy.types.Object, bpy.types.Object]:
    _require_mesh(obj)
    require_editable(obj)
    node_group = create_geometry_nodes_generate_goal_positions()

    choose = bpy.data.objects.new(f"{obj.name} - Choose", _icosphere_mesh(obj.name))
    choose.location = obj.matrix_world.translation if location is None else location
    move = bpy.data.objects.new(f"{obj.name} - Move", None)
    move.parent = choose
    for collection in obj.users_collection:
        collection.objects.link(choose)
        collection.objects.link(move)

    modifier = obj.modifiers.new(f"{MODIFIER_PREFIX} Goals", type="NODES")
    modifier.node_group = node_group
    set_modifier_input(modifier, "Choose", choose)
    set_modifier_input(modifier, "Move", move)
    return choose, move


def restrict_view(
    obj: bpy.types.Object, *, name: str | None = None, fit_vertices: bool = True
) -> bpy.types.Object:
    if obj.type not in {"MESH", "POINTCLOUD"}:
        raise InvalidInput(f"{obj.name} is neither a mesh nor a point cloud")
    require_editable(obj)
    index = _restrict_view_index(obj)
    fitting_box = _fitting_box(obj, index) if fit_vertices else None
    node_group = create_geometry_nodes_restrict_view()

    empty = bpy.data.objects.new(name or f"{obj.name} - View", None)
    empty.empty_display_type = "CUBE"
    if fitting_box is not None:
        empty.matrix_world = obj.matrix_world @ fitting_box
    for collection in obj.users_collection:
        collection.objects.link(empty)

    modifier = obj.modifiers.new(f"{MODIFIER_PREFIX} Restrict View", type="NODES")
    modifier.node_group = node_group
    set_modifier_input(modifier, "Object", empty)
    obj.modifiers.move(len(obj.modifiers) - 1, index)
    return empty


def add_drivers(obj: bpy.types.Object) -> list[bpy.types.Modifier]:
    sim_obj = _simulation_of(obj)
    require_editable(obj)
    driven = [
        modifier
        for modifier in obj.modifiers
        if modifier.name.startswith(MODIFIER_PREFIX)
        and getattr(modifier, "node_group", None) is not None
    ]
    for modifier in driven:
        add_modifier_drivers(sim_obj, modifier)
    return driven


def add_modifier_drivers(sim_obj: bpy.types.Object, modifier: bpy.types.Modifier):
    sockets = modifier.node_group.interface.items_tree
    for socket_name, (variable_name, data_path) in DRIVEN_SOCKETS.items():
        if socket_name in sockets:
            _drive_socket(
                modifier,
                sockets[socket_name].identifier,
                sim_obj,
                variable_name,
                data_path,
            )


def set_modifier_input(modifier: bpy.types.Modifier, socket_name: str, value):
    identifier = modifier.node_group.interface.items_tree[socket_name].identifier
    getattr(modifier.properties.inputs, identifier).value = value


def _drive_socket(
    modifier: bpy.types.Modifier,
    identifier: str,
    sim_obj: bpy.types.Object,
    variable_name: str,
    data_path: str,
):
    driver = getattr(modifier.properties.inputs, identifier).driver_add("value").driver
    driver.expression = variable_name
    for variable in list(driver.variables):
        driver.variables.remove(variable)
    variable = driver.variables.new()
    variable.name = variable_name
    variable.type = "SINGLE_PROP"
    target = variable.targets[0]
    target.fallback_value = 1
    target.data_path = data_path
    target.id_type = "OBJECT"
    target.id = sim_obj


def require_editable(obj: bpy.types.Object):
    if not obj.is_editable:
        raise InvalidInput(f"{obj.name} is linked from a library and can't be changed")


def _require_mesh(obj: bpy.types.Object):
    if obj.type != "MESH":
        raise InvalidInput(f"{obj.name} is not a mesh")


def _simulation_of(obj: bpy.types.Object) -> bpy.types.Object:
    uuid = obj.squishy_volumes.uuid
    for sim_obj in get_simulation_objects():
        if uuid and sim_obj.squishy_volumes.uuid == uuid and sim_obj != obj:
            return sim_obj
    raise InvalidInput(f"{obj.name} is not an input of a simulation")


def _icosphere_mesh(name: str) -> bpy.types.Mesh:
    mesh = bpy.data.meshes.new(f"{name} - Choose")
    sphere = bmesh.new()
    bmesh.ops.create_icosphere(sphere, subdivisions=2, radius=1.0)
    sphere.to_mesh(mesh)
    sphere.free()
    return mesh


def _restrict_view_index(obj: bpy.types.Object) -> int:
    loading = [
        index
        for index, modifier in enumerate(obj.modifiers)
        if modifier.type == "MESH_SEQUENCE_CACHE"
    ]
    return loading[-1] + 1 if loading else 0


def _bounds_before_modifier(
    obj: bpy.types.Object, index: int
) -> tuple[mathutils.Vector, mathutils.Vector]:
    later = [modifier for modifier in obj.modifiers[index:] if modifier.show_viewport]
    for modifier in later:
        modifier.show_viewport = False
    try:
        evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
        corners = [mathutils.Vector(corner) for corner in evaluated.bound_box]
    finally:
        for modifier in later:
            modifier.show_viewport = True
    return (
        mathutils.Vector(min(corner[i] for corner in corners) for i in range(3)),
        mathutils.Vector(max(corner[i] for corner in corners) for i in range(3)),
    )


def _fitting_box(obj: bpy.types.Object, index: int) -> mathutils.Matrix:
    bbox_min, bbox_max = _bounds_before_modifier(obj, index)
    if bbox_min == bbox_max:
        raise InvalidInput(f"{obj.name} has no geometry to fit the view to")
    size = (bbox_max - bbox_min) * 0.51
    return mathutils.Matrix.LocRotScale((bbox_min + bbox_max) / 2, None, size)
