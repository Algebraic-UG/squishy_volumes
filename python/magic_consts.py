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

# types of outputs
GRID_COLLIDER_DISTANCE = "GRID_COLLIDER_DISTANCE"
GRID_MOMENTUM_FREE = "GRID_MOMENTUM_FREE"
GRID_MOMENTUM_CONFORMED = "GRID_MOMENTUM_CONFORMED"
SOLID_PARTICLES = "SOLID_PARTICLES"
FLUID_PARTICLES = "FLUID_PARTICLES"
COLLIDER_SAMPLES = "COLLIDER_SAMPLES"
COLLIDER_MESH = "COLLIDER_MESH"
INPUT_MESH = "INPUT_MESH"

OUTPUT_TYPES = [
    GRID_COLLIDER_DISTANCE,
    GRID_MOMENTUM_FREE,
    GRID_MOMENTUM_CONFORMED,
    SOLID_PARTICLES,
    FLUID_PARTICLES,
    COLLIDER_SAMPLES,
    COLLIDER_MESH,
    INPUT_MESH,
]


# synchronisable attributes
BLENDED_MPM_INSTANCE_COLOR = "blended_mpm_instance_color"
BLENDED_MPM_ELASTIC_ENERGY = "blended_mpm_elastic_energy"
BLENDED_MPM_TRANSFORM = "blended_mpm_transform"
BLENDED_MPM_COLLIDER_INSIDE = "blended_mpm_collider_inside"
BLENDED_MPM_VELOCITY = "blended_mpm_velocity"
BLENDED_MPM_DISTANCE = "blended_mpm_distance"
BLENDED_MPM_NORMAL = "blended_mpm_normal"
BLENDED_MPM_MASS = "blended_mpm_mass"
BLENDED_MPM_PRESSURE = "blended_mpm_pressure"
BLENDED_MPM_REFERENCE_INDEX = "blended_mpm_reference_index"
BLENDED_MPM_REFERENCE_OFFSET = "blended_mpm_reference_offset"
BLENDED_MPM_INITIAL_LENGTH = "blended_mpm_initial_length"
BLENDED_MPM_BREAKING_FRAME = "blended_mpm_breaking_frame"
BLENDED_MPM_INITIAL_VOLUME = "blended_mpm_initial_volume"
