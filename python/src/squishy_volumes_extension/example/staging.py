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

import bpy
from mathutils import Euler, Vector

from .. import api
from .palette import assign_material, collider_material, particle_material


def new_example_collection(scene: bpy.types.Scene, name: str) -> bpy.types.Collection:
    collection = bpy.data.collections.new(name)
    scene.collection.children.link(collection)
    return collection


def move_to_collection(obj: bpy.types.Object, collection: bpy.types.Collection):
    for other in list(obj.users_collection):
        other.objects.unlink(obj)
    collection.objects.link(obj)


def add_camera(
    collection: bpy.types.Collection, name: str, location, target
) -> bpy.types.Object:
    camera = bpy.data.objects.new(name, bpy.data.cameras.new(name))
    camera.location = location
    direction = Vector(target) - Vector(location)
    camera.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
    collection.objects.link(camera)
    return camera


def add_sun(
    collection: bpy.types.Collection, name: str, rotation, strength: float
) -> bpy.types.Object:
    light = bpy.data.lights.new(name, type="SUN")
    light.energy = strength
    sun = bpy.data.objects.new(name, light)
    sun.rotation_euler = Euler(rotation)
    collection.objects.link(sun)
    return sun


def stage_particles(
    sim_obj: bpy.types.Object,
    obj: bpy.types.Object,
    collection: bpy.types.Collection,
    material_index: int,
    scale: float,
) -> bpy.types.Object:
    move_to_collection(obj, collection)
    output_obj = api.add_input(sim_obj, obj, api.PARTICLES)
    assert output_obj is not None
    material = particle_material(material_index)
    assign_material(obj, material)
    assign_material(output_obj, material)
    visualization = output_obj.modifiers["Squishy Volumes Output"]
    api.set_modifier_input(visualization, "Scale", scale)
    api.set_modifier_input(visualization, "Material", material)
    return output_obj


def stage_collider(
    sim_obj: bpy.types.Object,
    obj: bpy.types.Object,
    collection: bpy.types.Collection,
    material_index: int,
):
    move_to_collection(obj, collection)
    api.add_input(sim_obj, obj, api.COLLIDER)
    assign_material(obj, collider_material(material_index))
