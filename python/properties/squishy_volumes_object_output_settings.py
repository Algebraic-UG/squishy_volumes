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

from ..magic_consts import (
    SQUISHY_VOLUMES_ELASTIC_ENERGY,
    SQUISHY_VOLUMES_STATE,
    SQUISHY_VOLUMES_TRANSFORM,
    SQUISHY_VOLUMES_COLLIDER_INSIDE,
    SQUISHY_VOLUMES_INITIAL_POSITION,
    SQUISHY_VOLUMES_VELOCITY,
    SQUISHY_VOLUMES_DISTANCE,
    SQUISHY_VOLUMES_NORMAL,
    SQUISHY_VOLUMES_MASS,
    SQUISHY_VOLUMES_PRESSURE,
    SQUISHY_VOLUMES_INITIAL_VOLUME,
    COLLIDER_MESH,
    INPUT_MESH,
    GRID_COLLIDER_DISTANCE,
    GRID_MOMENTUM_FREE,
    GRID_MOMENTUM_CONFORMED,
    PARTICLES,
    COLLIDER_SAMPLES,
    OUTPUT_TYPES,
)


def optional_attributes_set_all(optional_attributes, value):
    optional_attributes.grid_collider_distances = value
    optional_attributes.grid_collider_normals = value

    optional_attributes.grid_momentum_masses = value
    optional_attributes.grid_momentum_velocities = value

    optional_attributes.particle_states = value
    optional_attributes.particle_masses = value
    optional_attributes.particle_initial_volumes = value
    optional_attributes.particle_initial_positions = value
    optional_attributes.particle_velocities = value
    optional_attributes.particle_transformations = value
    optional_attributes.particle_energies = value
    optional_attributes.particle_collider_insides = value

    optional_attributes.collider_normals = value
    optional_attributes.collider_velocities = value


def draw_object_attributes(layout, output_type, optional_attributes):
    if output_type in [COLLIDER_MESH, INPUT_MESH]:
        return

    layout.label(text="Please mouse-over for the exact identifier.")
    grid = layout.grid_flow(row_major=True, columns=2, even_columns=False)
    grid.label(text="Attribute")
    grid.label(text="Type")
    if output_type == GRID_COLLIDER_DISTANCE:
        grid.prop(optional_attributes, "grid_collider_distances")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "grid_collider_normals")
        grid.label(text="FLOAT_VECTOR")
    if output_type in [GRID_MOMENTUM_FREE, GRID_MOMENTUM_CONFORMED]:
        grid.prop(optional_attributes, "grid_momentum_masses")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "grid_momentum_velocities")
        grid.label(text="FLOAT_VECTOR")
    if output_type == PARTICLES:
        grid.prop(optional_attributes, "particle_states")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "particle_masses")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "particle_initial_volumes")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "particle_initial_positions")
        grid.label(text="FLOAT_VECTOR")
        grid.prop(optional_attributes, "particle_velocities")
        grid.label(text="FLOAT_VECTOR")
        grid.prop(optional_attributes, "particle_transformations")
        grid.label(text="FLOAT4X4")
        grid.prop(optional_attributes, "particle_energies")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "particle_collider_insides")
        grid.label(text="FLOAT")
    if output_type == COLLIDER_SAMPLES:
        grid.prop(optional_attributes, "collider_normals")
        grid.label(text="FLOAT_VECTOR")
        grid.prop(optional_attributes, "collider_velocities")
        grid.label(text="FLOAT_VECTOR")


class Squishy_Volumes_Object_Output_Settings(bpy.types.PropertyGroup):
    output_type: bpy.props.EnumProperty(
        name="Output Type",
        description="Depending on this, different attributes are synchronizable.",
        items=[(ty,) * 3 for ty in OUTPUT_TYPES],  # ty:ignore[invalid-argument-type]
        default=PARTICLES,
        options=set(),
    )  # type: ignore

    input_name: bpy.props.StringProperty(
        name="Original Input Name",
        description="Referenced for retrieving object-bound outputs.",
        options=set(),
    )  # type: ignore

    grid_collider_distances: bpy.props.BoolProperty(
        name="Distance",
        description=f"Attribute name: {SQUISHY_VOLUMES_DISTANCE}",
        default=True,
        options=set(),
    )  # type: ignore

    grid_collider_normals: bpy.props.BoolProperty(
        name="Normal",
        description=f"Attribute name: {SQUISHY_VOLUMES_NORMAL}",
        default=True,
        options=set(),
    )  # type: ignore

    grid_momentum_masses: bpy.props.BoolProperty(
        name="Masses",
        description=f"Attribute name: {SQUISHY_VOLUMES_MASS}",
        default=True,
        options=set(),
    )  # type: ignore

    grid_momentum_velocities: bpy.props.BoolProperty(
        name="Velocities",
        description=f"Attribute name: {SQUISHY_VOLUMES_VELOCITY}",
        default=True,
        options=set(),
    )  # type: ignore

    particle_states: bpy.props.BoolProperty(
        name="States",
        description=f"Attribute name: {SQUISHY_VOLUMES_STATE}",
        default=False,
        options=set(),
    )  # type: ignore

    particle_masses: bpy.props.BoolProperty(
        name="Masses",
        description=f"Attribute name: {SQUISHY_VOLUMES_MASS}",
        default=False,
        options=set(),
    )  # type: ignore

    particle_initial_positions: bpy.props.BoolProperty(
        name="Initial Positions",
        description=f"Attribute name: {SQUISHY_VOLUMES_INITIAL_POSITION}",
        default=True,
        options=set(),
    )  # type: ignore

    particle_initial_volumes: bpy.props.BoolProperty(
        name="Initial Volumes",
        description=f"Attribute name: {SQUISHY_VOLUMES_INITIAL_VOLUME}",
        default=False,
        options=set(),
    )  # type: ignore

    particle_velocities: bpy.props.BoolProperty(
        name="Velocites",
        description=f"Attribute name: {SQUISHY_VOLUMES_VELOCITY}",
        default=True,
        options=set(),
    )  # type: ignore

    particle_transformations: bpy.props.BoolProperty(
        name="Transformations",
        description=f"Attribute name: {SQUISHY_VOLUMES_TRANSFORM}",
        default=True,
        options=set(),
    )  # type: ignore

    particle_energies: bpy.props.BoolProperty(
        name="Energies",
        description=f"Attribute name: {SQUISHY_VOLUMES_ELASTIC_ENERGY}",
        default=True,
        options=set(),
    )  # type: ignore

    particle_collider_insides: bpy.props.BoolProperty(
        name="Collider Insides",
        description=f"Attribute name: {SQUISHY_VOLUMES_COLLIDER_INSIDE}_X",
        default=True,
        options=set(),
    )  # type: ignore

    collider_normals: bpy.props.BoolProperty(
        name="Normals",
        description=f"Attribute name: {SQUISHY_VOLUMES_NORMAL}",
        default=True,
        options=set(),
    )  # type: ignore

    collider_velocities: bpy.props.BoolProperty(
        name="Velocities",
        description=f"Attribute name: {SQUISHY_VOLUMES_VELOCITY}",
        default=True,
        options=set(),
    )  # type: ignore
