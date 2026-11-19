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

from .. import api

EXAMPLE_BOING_BLOCK = "Boing Block"


def setup_example_boing_block(context: bpy.types.Context):
    sim_obj = api.add_simulation(EXAMPLE_BOING_BLOCK, scene=context.scene)
    sim_props = sim_obj.squishy_volumes
    sim_uuid = sim_props.uuid

    sim_props.grid_node_size = 0.2
    sim_props.simulation_scale = 5
    sim_props.time_step = 0.005

    bpy.ops.mesh.primitive_cube_add(
        size=2,
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, 0),
        scale=(1, 1, 1),
        rotation=(0, 0, 0),
    )
    api.add_input(sim_obj, context.active_object, api.PARTICLES)

    bpy.ops.mesh.primitive_plane_add(
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -2),
        scale=(1, 1, 1),
        size=10,
    )
    api.add_input(sim_obj, context.active_object, api.COLLIDER)

    bpy.ops.scene.squishy_volumes_record_input_to_cache(
        "INVOKE_DEFAULT",
        uuid=sim_uuid,
        blocking=True,
        start_baking=False,
    )

    return sim_obj
