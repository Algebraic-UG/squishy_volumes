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

from .. import api

EXAMPLE_BENCHMARK = "Benchmark"


def setup_example_benchmark(context: bpy.types.Context):
    sim_obj = api.add_simulation(EXAMPLE_BENCHMARK, scene=context.scene)
    sim_props = sim_obj.squishy_volumes
    sim_uuid = sim_props.uuid

    sim_props.grid_node_size = 0.1
    sim_props.time_step = 0.001
    sim_props.bake_frames = 60

    sim_props.max_giga_bytes_on_disk = 15.0

    def add_monkey(location, rotation):
        bpy.ops.mesh.primitive_monkey_add(
            size=4,
            enter_editmode=False,
            align="WORLD",
            location=location,
            scale=(1, 1, 1),
            rotation=rotation,
        )
        monkey = context.active_object
        api.add_input(sim_obj, monkey, api.PARTICLES)
        monkey.hide_set(True)

    add_monkey(
        location=(-2, -2, 0),
        rotation=(math.radians(-45), 0, math.radians(135)),
    )
    add_monkey(
        location=(-2, 2, 0),
        rotation=(math.radians(-45), 0, math.radians(45)),
    )
    add_monkey(
        location=(2, 2, 0),
        rotation=(math.radians(-45), 0, math.radians(-45)),
    )
    add_monkey(
        location=(2, -2, 0),
        rotation=(math.radians(-45), 0, math.radians(-135)),
    )
    add_monkey(
        location=(-2, -2, 4),
        rotation=(math.radians(-45), 0, math.radians(135)),
    )
    add_monkey(
        location=(-2, 2, 4),
        rotation=(math.radians(-45), 0, math.radians(45)),
    )
    add_monkey(
        location=(2, 2, 4),
        rotation=(math.radians(-45), 0, math.radians(-45)),
    )
    add_monkey(
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
    api.add_input(sim_obj, context.active_object, api.COLLIDER)

    bpy.ops.mesh.primitive_torus_add(
        align="WORLD",
        location=(0, 0, -3),
        rotation=(0, 0, 0),
        major_radius=4,
        minor_radius=1,
    )
    api.add_input(sim_obj, context.active_object, api.COLLIDER)

    bpy.ops.mesh.primitive_plane_add(
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -10),
        scale=(1, 1, 1),
        size=100,
    )
    api.add_input(sim_obj, context.active_object, api.COLLIDER)

    bpy.ops.scene.squishy_volumes_record_input_to_cache(
        "INVOKE_DEFAULT",
        uuid=sim_uuid,
        blocking=True,
        start_baking=False,
    )

    return sim_obj
