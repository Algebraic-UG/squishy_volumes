# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Blended MPM extension.
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


def update_particle_size(self, _context):
    self.particle_size = self.grid_node_size * self.particle_factor


class Blended_MPM_Simulation_Settings(bpy.types.PropertyGroup):
    grid_node_size: bpy.props.FloatProperty(
        name="Grid Node Size",
        description="""The major discrete space resolution of the simulation.
Can be tricky to get right: Lower values grant higher fidelity
in the simulation but impacts performance and stability.

Overwrite the cache to manifest changes.""",
        default=0.5,
        min=0.001,
        precision=5,
        options=set(),
        update=update_particle_size,
    )  # type: ignore
    particle_factor: bpy.props.FloatProperty(
        name="Particle Factor",
        description="""Controls the particle size:

    particle_size = grid_node_size * particle_factor

The particles need to be smaller than the grid nodes to interact.

Overwrite the cache to manifest changes.""",
        default=0.5,
        max=1.0,
        min=0.1,
        update=update_particle_size,
    )  # type: ignore
    particle_size: bpy.props.FloatProperty(
        name="Particle Size",
        description="""Readonly. The minor discrete space resolution of the simulation.
This also can benefit fidelity with less adverse impact on performance.

Overwrite the cache to manifest changes.""",
        default=0.25,
        min=0.0005,
        precision=6,
    )  # type: ignore
    frames_per_second: bpy.props.IntProperty(
        name="Frames per Second",
        description="""Controls how many simulation steps end up as viewable frames per simulated second.
If blender's native FPS differs from this setting you'll get 'artifical' speedup or slowdown.

For example:
Given that blender's native FPS is set to 24 (default),
to achieve a 4x 'slowmotion' effect, you need to set this to 96.

Note that this also effects the interpretation of captured animations from input objects.

Overwrite the cache to manifest changes.""",
        default=24,
        min=1,
    )  # type: ignore
    gravity: bpy.props.FloatVectorProperty(
        name="Gravity",
        description="It is currently the only volumetric force and it is constant.",
        default=(0.0, 0.0, -9.8),
        options=set(),
    )  # type: ignore
