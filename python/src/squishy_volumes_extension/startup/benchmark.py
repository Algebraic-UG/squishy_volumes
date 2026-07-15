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
import uuid
import math

import tempfile
from pathlib import Path


from ..properties.squishy_volumes_simulation import (
    update_name,
    update_directory,
)

from ..properties.squishy_volumes_object_input_settings import (
    INPUT_TYPE_PARTICLES,
    INPUT_TYPE_COLLIDER,
)

from ..util import simulation_locked

STARTUP_BENCHMARK = "Benchmark"


def setup_startup_benchmark(context: bpy.types.Context):
    bpy.ops.scene.squishy_volumes_add_simulation("INVOKE_DEFAULT")  # ty: ignore[unresolved-attribute]

    simulation = context.scene.squishy_volumes_scene.simulations[-1]

    simulation.name = STARTUP_BENCHMARK
    simulation.directory = str(
        Path(tempfile.gettempdir()) / f"squishy_volumes_{STARTUP_BENCHMARK}"
    )

    simulation.grid_node_size = 0.1
    simulation.time_step = 0.001
    simulation.capture_frames = 1
    simulation.bake_frames = 60
    simulation.immediately_start_baking = False

    input_particles = []
    input_collider = []

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(-2, -2, 0),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(135)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(-2, 2, 0),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(45)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(2, 2, 0),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(-45)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(2, -2, 0),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(-135)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(-2, -2, 4),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(135)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(-2, 2, 4),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(45)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(2, 2, 4),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(-45)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_monkey_add(
        size=4,
        enter_editmode=False,
        align="WORLD",
        location=(2, -2, 4),
        scale=(1, 1, 1),
        rotation=(math.radians(-45), 0, math.radians(-135)),
    )
    input_particles.append(context.active_object.name)

    bpy.ops.mesh.primitive_cone_add(
        radius1=2,
        depth=4,
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -2),
        scale=(1, 1, 1),
    )
    input_collider.append(context.active_object.name)

    bpy.ops.mesh.primitive_torus_add(
        align="WORLD",
        location=(0, 0, -3),
        rotation=(0, 0, 0),
        major_radius=4,
        minor_radius=1,
    )
    input_collider.append(context.active_object.name)

    bpy.ops.mesh.primitive_plane_add(
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -10),
        scale=(1, 1, 1),
        size=100,
    )
    input_collider.append(context.active_object.name)

    for name in input_particles:
        obj = bpy.data.objects[name]
        obj.squishy_volumes_object.input_settings.input_type = INPUT_TYPE_PARTICLES
        obj.select_set(True)

    for name in input_collider:
        obj = bpy.data.objects[name]
        obj.squishy_volumes_object.input_settings.input_type = INPUT_TYPE_COLLIDER
        obj.select_set(True)

    bpy.ops.scene.squishy_volumes_add_input_objects()
    bpy.ops.scene.squishy_volumes_write_input_to_cache(  # ty:ignore[unresolved-attribute]
        uuid=simulation.uuid, blocking=True
    )

    for name in input_particles:
        bpy.ops.scene.squishy_volumes_add_output_objects(
            uuid=simulation.uuid,
            called_from_script=True,
            input_name=name,
        )

    simulation.immediately_start_baking = True
    bpy.ops.scene.squishy_volumes_bake_start_from_latest()
