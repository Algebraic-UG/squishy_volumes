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

# This code has been mostly generated with https://github.com/BrendanParmer/NodeToPython
# NodeToPython is licensed under the GPLv3 License.

import bpy

from ..magic_consts import SQUISHY_VOLUMES_INITIAL_POSITION


def create_material_display_uvw():
    mat = bpy.data.materials.new(name="Squishy Volumes Display UVW")
    mat.use_nodes = True

    def squishy_volumes_display_uvw_node_group():
        """Initialize Squishy Volumes Display UVW node group"""
        squishy_volumes_display_uvw = mat.node_tree

        # Start with a clean node tree
        for node in squishy_volumes_display_uvw.nodes:
            squishy_volumes_display_uvw.nodes.remove(node)
        squishy_volumes_display_uvw.color_tag = "NONE"
        squishy_volumes_display_uvw.description = ""
        squishy_volumes_display_uvw.default_group_node_width = 140
        # squishy_volumes_display_uvw interface

        # Initialize squishy_volumes_display_uvw nodes

        # Node Principled BSDF
        principled_bsdf = squishy_volumes_display_uvw.nodes.new(
            "ShaderNodeBsdfPrincipled"
        )
        principled_bsdf.name = "Principled BSDF"
        principled_bsdf.distribution = "MULTI_GGX"
        principled_bsdf.subsurface_method = "RANDOM_WALK"
        # Metallic
        principled_bsdf.inputs[1].default_value = 0.0
        # Roughness
        principled_bsdf.inputs[2].default_value = 0.5
        # IOR
        principled_bsdf.inputs[3].default_value = 1.5
        # Alpha
        principled_bsdf.inputs[4].default_value = 1.0
        # Normal
        principled_bsdf.inputs[5].default_value = (0.0, 0.0, 0.0)
        # Diffuse Roughness
        principled_bsdf.inputs[7].default_value = 0.0
        # Subsurface Weight
        principled_bsdf.inputs[8].default_value = 0.0
        # Subsurface Radius
        principled_bsdf.inputs[9].default_value = (
            1.0,
            0.20000000298023224,
            0.10000000149011612,
        )
        # Subsurface Scale
        principled_bsdf.inputs[10].default_value = 0.05000000074505806
        # Subsurface Anisotropy
        principled_bsdf.inputs[12].default_value = 0.0
        # Specular IOR Level
        principled_bsdf.inputs[13].default_value = 0.5
        # Specular Tint
        principled_bsdf.inputs[14].default_value = (1.0, 1.0, 1.0, 1.0)
        # Anisotropic
        principled_bsdf.inputs[15].default_value = 0.0
        # Anisotropic Rotation
        principled_bsdf.inputs[16].default_value = 0.0
        # Tangent
        principled_bsdf.inputs[17].default_value = (0.0, 0.0, 0.0)
        # Transmission Weight
        principled_bsdf.inputs[18].default_value = 0.0
        # Coat Weight
        principled_bsdf.inputs[19].default_value = 0.0
        # Coat Roughness
        principled_bsdf.inputs[20].default_value = 0.029999999329447746
        # Coat IOR
        principled_bsdf.inputs[21].default_value = 1.5
        # Coat Tint
        principled_bsdf.inputs[22].default_value = (1.0, 1.0, 1.0, 1.0)
        # Coat Normal
        principled_bsdf.inputs[23].default_value = (0.0, 0.0, 0.0)
        # Sheen Weight
        principled_bsdf.inputs[24].default_value = 0.0
        # Sheen Roughness
        principled_bsdf.inputs[25].default_value = 0.5
        # Sheen Tint
        principled_bsdf.inputs[26].default_value = (1.0, 1.0, 1.0, 1.0)
        # Emission Color
        principled_bsdf.inputs[27].default_value = (1.0, 1.0, 1.0, 1.0)
        # Emission Strength
        principled_bsdf.inputs[28].default_value = 0.0
        # Thin Film Thickness
        principled_bsdf.inputs[29].default_value = 0.0
        # Thin Film IOR
        principled_bsdf.inputs[30].default_value = 1.3300000429153442

        # Node Material Output
        material_output = squishy_volumes_display_uvw.nodes.new(
            "ShaderNodeOutputMaterial"
        )
        material_output.name = "Material Output"
        material_output.is_active_output = True
        material_output.target = "ALL"
        # Displacement
        material_output.inputs[2].default_value = (0.0, 0.0, 0.0)
        # Thickness
        material_output.inputs[3].default_value = 0.0

        # Node Attribute
        attribute = squishy_volumes_display_uvw.nodes.new("ShaderNodeAttribute")
        attribute.name = "Attribute"
        attribute.attribute_name = SQUISHY_VOLUMES_INITIAL_POSITION
        attribute.attribute_type = "GEOMETRY"

        # Node Separate XYZ
        separate_xyz = squishy_volumes_display_uvw.nodes.new("ShaderNodeSeparateXYZ")
        separate_xyz.name = "Separate XYZ"

        # Node Math
        math = squishy_volumes_display_uvw.nodes.new("ShaderNodeMath")
        math.name = "Math"
        math.hide = True
        math.operation = "WRAP"
        math.use_clamp = False
        # Value_001
        math.inputs[1].default_value = 0.0
        # Value_002
        math.inputs[2].default_value = 1.0

        # Node Math.001
        math_001 = squishy_volumes_display_uvw.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.hide = True
        math_001.operation = "WRAP"
        math_001.use_clamp = False
        # Value_001
        math_001.inputs[1].default_value = 0.0
        # Value_002
        math_001.inputs[2].default_value = 1.0

        # Node Math.002
        math_002 = squishy_volumes_display_uvw.nodes.new("ShaderNodeMath")
        math_002.name = "Math.002"
        math_002.hide = True
        math_002.operation = "WRAP"
        math_002.use_clamp = False
        # Value_001
        math_002.inputs[1].default_value = 0.0
        # Value_002
        math_002.inputs[2].default_value = 1.0

        # Node Combine Color
        combine_color = squishy_volumes_display_uvw.nodes.new("ShaderNodeCombineColor")
        combine_color.name = "Combine Color"
        combine_color.mode = "RGB"

        # Node Vector Math
        vector_math = squishy_volumes_display_uvw.nodes.new("ShaderNodeVectorMath")
        vector_math.name = "Vector Math"
        vector_math.operation = "SCALE"
        # Scale
        vector_math.inputs[3].default_value = 1.0

        # Set locations
        principled_bsdf.location = (-180.0, 160.0)
        material_output.location = (100.0, 160.0)
        attribute.location = (-1080.0, 160.0)
        separate_xyz.location = (-720.0, 160.0)
        math.location = (-540.0, 140.0)
        math_001.location = (-540.0, 100.0)
        math_002.location = (-540.0, 60.0)
        combine_color.location = (-360.0, 160.0)
        vector_math.location = (-900.0, 160.0)

        # Set dimensions
        principled_bsdf.width, principled_bsdf.height = 240.0, 100.0
        material_output.width, material_output.height = 140.0, 100.0
        attribute.width, attribute.height = 140.0, 100.0
        separate_xyz.width, separate_xyz.height = 140.0, 100.0
        math.width, math.height = 140.0, 100.0
        math_001.width, math_001.height = 140.0, 100.0
        math_002.width, math_002.height = 140.0, 100.0
        combine_color.width, combine_color.height = 140.0, 100.0
        vector_math.width, vector_math.height = 140.0, 100.0

        # Initialize squishy_volumes_display_uvw links

        # principled_bsdf.BSDF -> material_output.Surface
        squishy_volumes_display_uvw.links.new(
            principled_bsdf.outputs[0], material_output.inputs[0]
        )
        # vector_math.Vector -> separate_xyz.Vector
        squishy_volumes_display_uvw.links.new(
            vector_math.outputs[0], separate_xyz.inputs[0]
        )
        # separate_xyz.X -> math.Value
        squishy_volumes_display_uvw.links.new(separate_xyz.outputs[0], math.inputs[0])
        # separate_xyz.Y -> math_001.Value
        squishy_volumes_display_uvw.links.new(
            separate_xyz.outputs[1], math_001.inputs[0]
        )
        # separate_xyz.Z -> math_002.Value
        squishy_volumes_display_uvw.links.new(
            separate_xyz.outputs[2], math_002.inputs[0]
        )
        # math.Value -> combine_color.Red
        squishy_volumes_display_uvw.links.new(math.outputs[0], combine_color.inputs[0])
        # math_001.Value -> combine_color.Green
        squishy_volumes_display_uvw.links.new(
            math_001.outputs[0], combine_color.inputs[1]
        )
        # math_002.Value -> combine_color.Blue
        squishy_volumes_display_uvw.links.new(
            math_002.outputs[0], combine_color.inputs[2]
        )
        # combine_color.Color -> principled_bsdf.Base Color
        squishy_volumes_display_uvw.links.new(
            combine_color.outputs[0], principled_bsdf.inputs[0]
        )
        # attribute.Vector -> vector_math.Vector
        squishy_volumes_display_uvw.links.new(
            attribute.outputs[1], vector_math.inputs[0]
        )

        return squishy_volumes_display_uvw

    squishy_volumes_display_uvw_node_group()

    return mat
