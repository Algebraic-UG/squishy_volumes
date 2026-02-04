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
import json

from .properties.squishy_volumes_object import get_input_objects


def create_setup_json(simulation):
    scene = bpy.context.scene

    grid_node_size = simulation.grid_node_size
    simulation_scale = simulation.simulation_scale
    frames_per_second = simulation.frames_per_second
    domain_min = [
        simulation.domain_min[0],
        simulation.domain_min[1],
        simulation.domain_min[2],
    ]
    domain_max = [
        simulation.domain_max[0],
        simulation.domain_max[1],
        simulation.domain_max[2],
    ]

    consts = {
        "grid_node_size": grid_node_size,
        "simulation_scale": simulation_scale,
        "frames_per_second": frames_per_second,
        "domain_min": domain_min,
        "domain_max": domain_max,
    }

    objects = []

    for obj in get_input_objects(simulation):
        name = obj.name
        ty = obj.squishy_volumes_object.object_type
        objects.append(
            {
                "name": name,
                "ty": ty,
            }
        )

    return json.dumps(
        {
            "consts": consts,
            "objects": objects,
        }
    )
