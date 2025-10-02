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

from ..properties.util import get_simulation_specific_settings, is_some_and

# these have to match the enum in core::api::ObjectSettings
OBJECT_ENUM_SOLID = "Solid"
OBJECT_ENUM_FLUID = "Fluid"
OBJECT_ENUM_COLLIDER = "Collider"


def get_input_objects_type(simulation, input_type):
    return [
        obj
        for obj in bpy.data.objects
        if is_some_and(
            get_simulation_specific_settings(simulation, obj),
            lambda settings: settings.object_enum == input_type,
        )
    ]


def get_input_solids(simulation):
    return get_input_objects_type(simulation, OBJECT_ENUM_SOLID)


def get_input_colliders(simulation):
    return get_input_objects_type(simulation, OBJECT_ENUM_COLLIDER)


class Squishy_Volumes_Object_Settings(bpy.types.PropertyGroup):
    simulation_uuid: bpy.props.StringProperty(
        name="Simulation UUID",
        description="Backreference to the simulation these settings are meant for.",
        default="unassigned",
        options=set(),
    )  # type: ignore

    density: bpy.props.FloatProperty(
        name="Density",
        description="""How dense/heavy the object should be. Unit: kg / m^3.
A few examples: Sponge 100, Water 1000, Earth 5000, Gold 19000.""",
        default=1000.0,
        min=1.0,
        max=20000.0,
        precision=1,
        options=set(),
    )  # type: ignore

    youngs_modulus: bpy.props.FloatProperty(
        name="Young's Modulus",
        description="""How stiff the object should be. Unit: Pa.
A few examples: Fat 1k, Rubber 1M, Iron 200G.
This directly influences the strength of the elastic response,
so higher values mandate a smaller time step.""",
        default=10000.0,
        min=1.0,
        max=1000000000000.0,
        precision=1,
        options=set(),
    )  # type: ignore

    poissons_ratio: bpy.props.FloatProperty(
        name="Poisson's Ratio",
        description="""How volume-conserving the object should be. Unit: None.
0 is not conserving at all, 0.49 is almost perfectly conserving.
In other words, how much the object bulges and contracts under
compression and streching.""",
        default=0.3,
        min=0.0,
        max=0.49,
        precision=1,
        options=set(),
    )  # type: ignore

    exponent: bpy.props.IntProperty(
        name="Exponent",
        description="""How quickly the fluid responds to compression.
With 2 it's still possible to get away with a fair amount of compression.
With 7 it's more like an incompressible fluid like water.""",
        default=7,
        min=2,
        max=10,
        options=set(),
    )  # type: ignore

    bulk_modulus: bpy.props.FloatProperty(
        name="Bulk Modulus",
        description="""Scales the elastic response of the liquid.""",
        default=1000.0,
        min=0.0,
        max=10000.0,
        precision=1,
        options=set(),
    )  # type: ignore

    use_viscosity: bpy.props.BoolProperty(
        name="Use Dynamic Viscosity",
        description="""TODO""",
        default=False,
        options=set(),
    )  # type: ignore
    viscosity: bpy.props.FloatProperty(
        name="Dynamic Viscosity",
        description="""TODO""",
        default=0.0,
        min=0.0,
        max=1000.0,
        precision=3,
        options=set(),
    )  # type: ignore

    dilation: bpy.props.FloatProperty(
        name="Dilation",
        description="""Assume an initial uniform dilation of the input geometry.

Less than one means it's compressed.
More than one means it's streched.

This will create a simulation matter that is under stress.
When simulating, the matter will expand or contract.

This also affects the spacing with witch the geometry is sampled.
The samples are spaced so that they will match an undilated geometry:
more samples under compression and less under streching.""",
        default=1.0,
        min=0.01,
        max=10.0,
        precision=2,
        options=set(),
    )  # type: ignore

    randomness: bpy.props.FloatProperty(
        name="Randomness",
        description="""Add a random offset to the sampled positions.""",
        default=0.0,
        min=0.0,
        max=1.0,
        precision=2,
        options=set(),
    )  # type: ignore

    sticky_factor: bpy.props.FloatProperty(
        name="Sticky Factor",
        description="""How sticky the collider object should be. Unit: None.
1 is the maximum, very sticky, 0 isn't sticky at all.""",
        default=0.0,
        min=0.0,
        max=1.0,
        precision=1,
        options=set(),
    )  # type: ignore

    friction_factor: bpy.props.FloatProperty(
        name="Friction Factor",
        description="""How much the rigid object resists sliding. Unit: None.
1 is quite resistive, 0 is slippery.""",
        default=0.3,
        min=0.0,
        max=10.0,
        precision=1,
        options=set(),
    )  # type: ignore

    use_sand_alpha: bpy.props.BoolProperty(
        name="Use Sand Alpha",
        description="""TODO""",
        default=False,
        options=set(),
    )  # type: ignore
    sand_alpha: bpy.props.FloatProperty(
        name="Sand Alpha",
        description="""TODO""",
        default=0.0,
        min=0.0,
        max=10.0,
        precision=3,
        options=set(),
    )  # type: ignore

    object_enum: bpy.props.EnumProperty(
        items=[
            (
                OBJECT_ENUM_SOLID,
                OBJECT_ENUM_SOLID,
                "Assume elastic solid matter within this mesh.",
            ),
            (
                OBJECT_ENUM_FLUID,
                OBJECT_ENUM_FLUID,
                "Assume fluid matter within this mesh.",
            ),
            (
                OBJECT_ENUM_COLLIDER,
                OBJECT_ENUM_COLLIDER,
                "Assume this to be a passive collider.",
            ),
        ],
        name="Type",
        description="""Object type, either (deformable) solid, fluid, or collider.
Depending on the type, further settings are available.""",
        default=OBJECT_ENUM_SOLID,
        options=set(),
    )  # type: ignore

    initial_linear_velocity: bpy.props.FloatVectorProperty(
        name="Initial Linear Velocity",
        description="The initial linear velocity of this object. Unit: m/s.",
        default=(0.0, 0.0, 0.0),
        options=set(),
    )  # type: ignore

    initial_angular_velocity: bpy.props.FloatVectorProperty(
        name="Initial Angular Velocity",
        description="""The initial angular velocity of this object. Unit: radians/s.
This is with respect to the object's location/center.""",
        default=(0.0, 0.0, 0.0),
        options=set(),
    )  # type: ignore
