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
from mathutils import Vector

from .staging import stage_example

EXAMPLE_PULL_APART = "Pull Apart"
PULL_FRAMES = 24


def setup_example_pull_apart(context: bpy.types.Context):
    import squishy_volumes as sv

    sim_obj = sv.add_simulation(EXAMPLE_PULL_APART, scene=context.scene)
    sim_props = sim_obj.squishy_volumes

    sim_props.grid_node_size = 0.2
    sim_props.simulation_scale = 5
    sim_props.time_step = 0.005
    sim_props.gravity = (0.0, 0.0, 0.0)

    bpy.ops.mesh.primitive_cube_add(size=2, location=(0, 0, 0))
    cube = context.active_object
    output_obj = sv.add_input(sim_obj, cube, sv.PARTICLES)
    sv.set_modifier_input(output_obj.modifiers["Squishy Volumes Output"], "Scale", 0.1)

    stage_example(
        context.scene,
        sim_obj,
        camera_location=(0.0, -12.0, 4.0),
        camera_target=(0.0, 0.0, 0.0),
    )

    for side in (1.0, -1.0):
        choose, move = sv.add_goals(cube, location=Vector((2.0 * side, 0.0, 0.0)))
        choose.scale = (2.0, 2.0, 2.0)
        move.keyframe_insert("location", frame=1)
        move.location.x = 0.5 * side
        move.keyframe_insert("location", frame=PULL_FRAMES)

    sv.set_capture_range(sim_obj, 1, PULL_FRAMES, scene=context.scene)
    return sim_obj
