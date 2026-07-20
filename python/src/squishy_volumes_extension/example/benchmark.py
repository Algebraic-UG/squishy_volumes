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


from ..squishy_volumes_properties import (
    update_directory,
)

from ..squishy_volumes_properties import (
    INPUT_TYPE_PARTICLES,
    INPUT_TYPE_COLLIDER,
    get_simulation_object_with_uuid,
    get_input_objects_with_uuid,
)

from ..magic_consts import PARTICLES

from ..util import simulation_locked

EXAMPLE_BENCHMARK = "Benchmark"


def setup_example_benchmark(context: bpy.types.Context):
    sim_uuid = str(uuid.uuid4())
    bpy.ops.scene.squishy_volumes_add_simulation(  # ty:ignore[unresolved-attribute]
        "INVOKE_DEFAULT", name=EXAMPLE_BENCHMARK, uuid=sim_uuid
    )

    sim_obj = get_simulation_object_with_uuid(sim_uuid)
    sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

    sim_props.grid_node_size = 0.1
    sim_props.time_step = 0.001
    sim_props.capture_frames = 1
    sim_props.bake_frames = 60

    def add_particles(obj: bpy.types.Object):
        obj.squishy_volumes.input_type = INPUT_TYPE_PARTICLES  # ty:ignore[unresolved-attribute]
        obj.squishy_volumes.add_default_generation = True  # ty:ignore[unresolved-attribute]
        bpy.ops.scene.squishy_volumes_add_input_object(  # ty:ignore[unresolved-attribute]
            "INVOKE_DEFAULT",
            uuid=sim_uuid,
            name=obj.name,
        )
        obj.hide_set(True)

    def add_collider(obj):
        obj.squishy_volumes.input_type = INPUT_TYPE_COLLIDER
        obj.squishy_volumes.add_default_generation = True
        bpy.ops.scene.squishy_volumes_add_input_object(  # ty:ignore[unresolved-attribute]
            "INVOKE_DEFAULT",
            uuid=sim_uuid,
            name=obj.name,
        )

    def add_monekey(location, rotation):
        bpy.ops.mesh.primitive_monkey_add(
            size=4,
            enter_editmode=False,
            align="WORLD",
            location=location,
            scale=(1, 1, 1),
            rotation=rotation,
        )
        add_particles(context.active_object)  # ty:ignore[invalid-argument-type]

    add_monekey(
        location=(-2, -2, 0),
        rotation=(math.radians(-45), 0, math.radians(135)),
    )
    add_monekey(
        location=(-2, 2, 0),
        rotation=(math.radians(-45), 0, math.radians(45)),
    )
    add_monekey(
        location=(2, 2, 0),
        rotation=(math.radians(-45), 0, math.radians(-45)),
    )
    add_monekey(
        location=(2, -2, 0),
        rotation=(math.radians(-45), 0, math.radians(-135)),
    )
    add_monekey(
        location=(-2, -2, 4),
        rotation=(math.radians(-45), 0, math.radians(135)),
    )
    add_monekey(
        location=(-2, 2, 4),
        rotation=(math.radians(-45), 0, math.radians(45)),
    )
    add_monekey(
        location=(2, 2, 4),
        rotation=(math.radians(-45), 0, math.radians(-45)),
    )
    add_monekey(
        location=(2, -2, 4),
        rotation=(math.radians(-45), 0, math.radians(-135)),
    )

    bpy.ops.mesh.primitive_cone_add(
        radius1=2,
        depth=4,
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -2),
        scale=(1, 1, 1),
    )
    add_collider(context.active_object)

    bpy.ops.mesh.primitive_torus_add(
        align="WORLD",
        location=(0, 0, -3),
        rotation=(0, 0, 0),
        major_radius=4,
        minor_radius=1,
    )
    add_collider(context.active_object)

    bpy.ops.mesh.primitive_plane_add(
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -10),
        scale=(1, 1, 1),
        size=100,
    )
    add_collider(context.active_object)

    bpy.ops.scene.squishy_volumes_record_input_to_cache(  # ty:ignore[unresolved-attribute]
        "INVOKE_DEFAULT",
        uuid=sim_uuid,
        blocking=True,
        start_baking=False,
    )

    for input_obj in get_input_objects_with_uuid(sim_uuid):
        if input_obj.squishy_volumes.input_type != INPUT_TYPE_PARTICLES:  # ty:ignore[unresolved-attribute]
            continue
        bpy.ops.scene.squishy_volumes_add_output_object(  # ty:ignore[unresolved-attribute]
            "INVOKE_DEFAULT",
            uuid=sim_uuid,
            input_name=input_obj.name,
            output_name=f"{input_obj.name} - Output",
            add_default_visualization=True,
            output_type=PARTICLES,
            grid_collider_bits=False,
            grid_masses=False,
            grid_velocities=False,
            particle_flags=False,
            particle_masses=False,
            particle_initial_volumes=False,
            particle_initial_positions=True,
            particle_velocities=False,
            particle_sizes=True,
            particle_transformations=True,
            particle_energies=False,
            particle_collider_bits=False,
        )
    bpy.ops.scene.squishy_volumes_bake_start_from_latest(  # ty:ignore[unresolved-attribute]
        "INVOKE_DEFAULT", uuid=sim_uuid
    )
