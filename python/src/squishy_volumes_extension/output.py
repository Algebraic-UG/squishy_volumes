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


import bpy  # ty: ignore[unresolved-import]

from .assets import (
    create_geometry_nodes_grid,
    create_geometry_nodes_particles,
    create_material_display_uvw,
)
from .bridge import SimulationHandle
from .drivers import add_drivers


def create_default_visualization(sim_obj, output_obj):
    output_props = output_obj.squishy_volumes
    modifier = output_obj.modifiers.new(
        "Squishy Volumes Default Visualization", type="NODES"
    )

    modifier.node_group = create_geometry_nodes_particles()
    if bpy.app.version[0] == 5 and bpy.app.version[1] < 2:
        modifier["Socket_9"] = create_material_display_uvw()
        modifier["Socket_12"] = bpy.data.objects.get(output_props.input_name)
    else:
        modifier.properties.inputs.Socket_9.value = create_material_display_uvw()
        modifier.properties.inputs.Socket_12.value = bpy.data.objects.get(
            output_props.input_name
        )

    add_drivers(sim_obj, modifier)
