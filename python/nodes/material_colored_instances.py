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

# This code has been mostly generated with https://github.com/BrendanParmer/NodeToPython
# NodeToPython is licensed under the GPLv3 License.

import bpy

from ..magic_consts import BLENDED_MPM_INSTANCE_COLOR


def create_material_colored_instances():
    mat = bpy.data.materials.new(name="Blended MPM Colored Instances")
    mat.use_nodes = True

    # initialize Blended MPM Colored Instances node group
    def blended_mpm_colored_instances_node_group():
        blended_mpm_colored_instances = mat.node_tree
        # start with a clean node tree
        for node in blended_mpm_colored_instances.nodes:
            blended_mpm_colored_instances.nodes.remove(node)
        blended_mpm_colored_instances.color_tag = "NONE"
        blended_mpm_colored_instances.description = ""
        blended_mpm_colored_instances.default_group_node_width = 140

        # blended_mpm_colored_instances interface

        # initialize blended_mpm_colored_instances nodes
        # node Attribute
        attribute = blended_mpm_colored_instances.nodes.new("ShaderNodeAttribute")
        attribute.name = "Attribute"
        attribute.attribute_name = BLENDED_MPM_INSTANCE_COLOR
        attribute.attribute_type = "INSTANCER"

        # node Principled BSDF
        principled_bsdf = blended_mpm_colored_instances.nodes.new(
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
        # Weight
        principled_bsdf.inputs[6].default_value = 0.0
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
        # Subsurface IOR
        principled_bsdf.inputs[11].default_value = 1.399999976158142
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

        # node Material Output
        material_output = blended_mpm_colored_instances.nodes.new(
            "ShaderNodeOutputMaterial"
        )
        material_output.name = "Material Output"
        material_output.is_active_output = True
        material_output.target = "ALL"
        # Displacement
        material_output.inputs[2].default_value = (0.0, 0.0, 0.0)
        # Thickness
        material_output.inputs[3].default_value = 0.0

        # Set locations
        attribute.location = (0.0, 0.0)
        principled_bsdf.location = (300.0, 0.0)
        material_output.location = (600.0, 0.0)

        # Set dimensions
        attribute.width, attribute.height = 250.0, 100.0
        principled_bsdf.width, principled_bsdf.height = 240.0, 100.0
        material_output.width, material_output.height = 140.0, 100.0

        # initialize blended_mpm_colored_instances links
        # attribute.Color -> principled_bsdf.Base Color
        blended_mpm_colored_instances.links.new(
            attribute.outputs[0], principled_bsdf.inputs[0]
        )
        # principled_bsdf.BSDF -> material_output.Surface
        blended_mpm_colored_instances.links.new(
            principled_bsdf.outputs[0], material_output.inputs[0]
        )
        return blended_mpm_colored_instances

    blended_mpm_colored_instances_node_group()

    return mat
