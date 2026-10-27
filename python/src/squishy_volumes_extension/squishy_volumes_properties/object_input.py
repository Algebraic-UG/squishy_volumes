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

INPUT_TYPE_PARTICLES = "Particles"
INPUT_TYPE_COLLIDER = "Collider"

PARTICLE_ATTRIBUTE_MAPPING = {
    "Flags": ["Flags"],
    "Density": ["Density"],
    "Initial Size": ["InitialSize"],
    "Viscosity Dynamic": ["ViscosityDynamic"],
    "Viscosity Bulk": ["ViscosityBulk"],
    "Youngs Modulus": ["YoungsModulus"],
    "Poissons Ratio": ["PoissonsRatio"],
    "Sand Alpha": ["SandAlpha"],
    "Bulk Modulus": ["BulkModulus"],
    "Exponent": ["Exponent"],
    "Elastic Energy": ["ElasticEnergy"],
    "Collider Bits": ["ColliderBits"],
    # "Position": ["Position"],
    "Position Gradient": [
        "PositionGradientX",
        "PositionGradientY",
        "PositionGradientZ",
    ],
    "Velocity": ["Velocity"],
    "Velocity Gradient": [
        "VelocityGradientX",
        "VelocityGradientY",
        "VelocityGradientZ",
    ],
    "Initial Position": ["InitialPosition"],
    "Goal Position": ["GoalPosition"],
}


class Squishy_Volumes_Alembic_Attribute(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty()  # type: ignore
    enabled: bpy.props.BoolProperty()  # type: ignore


class Squishy_Volumes_Properties_Input(bpy.types.PropertyGroup):
    input_type: bpy.props.EnumProperty(
        items=[
            (INPUT_TYPE_PARTICLES,) * 3,
            (INPUT_TYPE_COLLIDER,) * 3,
        ],
        name="Type",
        description=f"""Simulation type of the object:
{INPUT_TYPE_PARTICLES}: point clouds that will be simulated, solid, fluid, etc.
{INPUT_TYPE_COLLIDER}:  passive (animated) mesh that is 100% user controlled.""",
        default=INPUT_TYPE_PARTICLES,
        options=set(),
    )  # type: ignore

    add_default_generation: bpy.props.BoolProperty(
        name="Add Default Generation",
        description="""Add default generating Geometry Nodes?

Squishy Volumes needs certain attributes on the input objects.
The default generation creates them.

It does not matter how the attributes are created, you can customize this.
For example: half solid, half fluid, or something like this.""",
        default=True,
    )  # type:ignore

    alembic_attributes: bpy.props.CollectionProperty(
        type=Squishy_Volumes_Alembic_Attribute,
        name="Alembic Attributes",
        description="A list of attributes that will be written to and read from the alembic files.",
        options=set(),
    )  # type: ignore
    selected_alembic_attribute: bpy.props.IntProperty(
        name="Selected Alembic Attribute",
        description="The selected alembic attribute can be removed.",
        options=set(),
    )  # type: ignore
