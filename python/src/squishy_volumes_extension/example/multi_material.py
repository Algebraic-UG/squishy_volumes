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
from .staging import stage_example

EXAMPLE_MULTI_MATERIAL = "Multi Material"


def setup_example_multi_material(context: bpy.types.Context):
    sim_obj = api.add_simulation(EXAMPLE_MULTI_MATERIAL, scene=context.scene)
    sim_props = sim_obj.squishy_volumes

    sim_props.grid_node_size = 0.2
    sim_props.time_step = 0.005

    bpy.ops.mesh.primitive_cube_add(size=2, location=(0, 0, 0))
    solid_output = api.add_input(sim_obj, context.active_object, api.PARTICLES)
    api.set_modifier_input(
        solid_output.modifiers["Squishy Volumes Output"], "Scale", 0.1
    )

    bpy.ops.mesh.primitive_uv_sphere_add(radius=1, location=(0, 0, 3))
    fluid = context.active_object
    fluid_output = api.add_input(sim_obj, fluid, api.PARTICLES)
    api.set_modifier_input(fluid.modifiers["Squishy Volumes Input"], "Type", "Fluid")
    api.set_modifier_input(
        fluid_output.modifiers["Squishy Volumes Output"], "Scale", 0.1
    )

    bpy.ops.mesh.primitive_plane_add(size=6, location=(0, 0, -2))
    api.add_input(sim_obj, context.active_object, api.COLLIDER)

    stage_example(
        context.scene,
        sim_obj,
        camera_location=(12.0, -12.0, 6.0),
        camera_target=(0.0, 0.0, 1.2),
    )
    return sim_obj
