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
SQUISHY_VOLUMES_INSTANCE_COLOR = "squishy_volumes_instance_color"
SQUISHY_VOLUMES_ELASTIC_ENERGY = "squishy_volumes_elastic_energy"
SQUISHY_VOLUMES_TRANSFORM = "squishy_volumes_transform"
SQUISHY_VOLUMES_COLLIDER_INSIDE = "squishy_volumes_collider_inside"
SQUISHY_VOLUMES_VELOCITY = "squishy_volumes_velocity"
SQUISHY_VOLUMES_DISTANCE = "squishy_volumes_distance"
SQUISHY_VOLUMES_NORMAL = "squishy_volumes_normal"
SQUISHY_VOLUMES_MASS = "squishy_volumes_mass"
SQUISHY_VOLUMES_PRESSURE = "squishy_volumes_pressure"
SQUISHY_VOLUMES_REFERENCE_INDEX = "squishy_volumes_reference_index"
SQUISHY_VOLUMES_REFERENCE_OFFSET = "squishy_volumes_reference_offset"
SQUISHY_VOLUMES_INITIAL_LENGTH = "squishy_volumes_initial_length"
SQUISHY_VOLUMES_BREAKING_FRAME = "squishy_volumes_breaking_frame"
SQUISHY_VOLUMES_INITIAL_VOLUME = "squishy_volumes_initial_volume"
