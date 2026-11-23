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
from mathutils import Euler, Vector

from .. import api
from .palette import assign_material, collider_material, particle_material


def stage_example(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object, *, camera_location, camera_target
) -> bpy.types.Object:
    collection = bpy.data.collections.new(sim_obj.name)
    scene.collection.children.link(collection)

    inputs = api.input_objects(sim_obj)
    particles = [
        obj for obj in inputs if obj.squishy_volumes.input_type == api.PARTICLES
    ]
    colliders = [
        obj for obj in inputs if obj.squishy_volumes.input_type == api.COLLIDER
    ]
    for index, input_obj in enumerate(particles):
        _stage_particles(sim_obj, input_obj, collection, particle_material(index))
    for index, input_obj in enumerate(colliders):
        _move_to_collection(input_obj, collection)
        assign_material(input_obj, collider_material(index))

    camera = _add_camera(
        collection, f"{sim_obj.name} Camera", camera_location, camera_target
    )
    _add_sun(collection, f"{sim_obj.name} Light")
    return camera


def _stage_particles(
    sim_obj: bpy.types.Object,
    input_obj: bpy.types.Object,
    collection: bpy.types.Collection,
    material: bpy.types.Material,
):
    _move_to_collection(input_obj, collection)
    assign_material(input_obj, material)
    for output_obj in _outputs_of(sim_obj, input_obj):
        _move_to_collection(output_obj, collection)
        assign_material(output_obj, material)
        visualization = output_obj.modifiers.get("Squishy Volumes Output")
        if visualization is not None:
            api.set_modifier_input(visualization, "Material", material)


def _outputs_of(
    sim_obj: bpy.types.Object, input_obj: bpy.types.Object
) -> list[bpy.types.Object]:
    return [
        obj
        for obj in bpy.data.objects
        if any(
            modifier.type == "MESH_SEQUENCE_CACHE"
            and modifier.cache_file == sim_obj.squishy_volumes.cache_file
            and modifier.object_path == f"/{input_obj.name}"
            for modifier in obj.modifiers
        )
    ]


def _move_to_collection(obj: bpy.types.Object, collection: bpy.types.Collection):
    for other in list(obj.users_collection):
        other.objects.unlink(obj)
    collection.objects.link(obj)


def _add_camera(
    collection: bpy.types.Collection, name: str, location, target
) -> bpy.types.Object:
    camera = bpy.data.objects.new(name, bpy.data.cameras.new(name))
    camera.location = location
    direction = Vector(target) - Vector(location)
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    collection.objects.link(camera)
    return camera


def _add_sun(collection: bpy.types.Collection, name: str) -> bpy.types.Object:
    light = bpy.data.lights.new(name, type="SUN")
    light.energy = 3.0
    sun = bpy.data.objects.new(name, light)
    sun.rotation_euler = Euler((math.radians(40), 0.0, math.radians(30)))
    collection.objects.link(sun)
    return sun
