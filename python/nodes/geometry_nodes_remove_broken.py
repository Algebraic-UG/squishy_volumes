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

from ..magic_consts import BLENDED_MPM_BREAKING_FRAME


def create_geometry_nodes_remove_broken():
    # initialize blended_mpm_remove_broken node group
    def blended_mpm_remove_broken_node_group():
        blended_mpm_remove_broken = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Remove Broken"
        )

        blended_mpm_remove_broken.color_tag = "NONE"
        blended_mpm_remove_broken.description = ""
        blended_mpm_remove_broken.default_group_node_width = 140

        blended_mpm_remove_broken.is_modifier = True

        # blended_mpm_remove_broken interface
        # Socket Geometry
        geometry_socket = blended_mpm_remove_broken.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = blended_mpm_remove_broken.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # initialize blended_mpm_remove_broken nodes
        # node Delete Geometry
        delete_geometry = blended_mpm_remove_broken.nodes.new(
            "GeometryNodeDeleteGeometry"
        )
        delete_geometry.name = "Delete Geometry"
        delete_geometry.domain = "EDGE"
        delete_geometry.mode = "EDGE_FACE"

        # node Named Attribute.006
        named_attribute_006 = blended_mpm_remove_broken.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_006.name = "Named Attribute.006"
        named_attribute_006.data_type = "INT"
        # Name
        named_attribute_006.inputs[0].default_value = BLENDED_MPM_BREAKING_FRAME

        # node Math.002
        math_002 = blended_mpm_remove_broken.nodes.new("ShaderNodeMath")
        math_002.name = "Math.002"
        math_002.operation = "LESS_THAN"
        math_002.use_clamp = False

        # node Scene Time
        scene_time = blended_mpm_remove_broken.nodes.new("GeometryNodeInputSceneTime")
        scene_time.name = "Scene Time"
        scene_time.outputs[0].hide = True

        # node Boolean Math
        boolean_math = blended_mpm_remove_broken.nodes.new("FunctionNodeBooleanMath")
        boolean_math.name = "Boolean Math"
        boolean_math.operation = "AND"

        # node Boolean Math.001
        boolean_math_001 = blended_mpm_remove_broken.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_001.name = "Boolean Math.001"
        boolean_math_001.operation = "NOT"

        # node Group Input
        group_input = blended_mpm_remove_broken.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

        # node Group Output
        group_output = blended_mpm_remove_broken.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # Set locations
        delete_geometry.location = (1360.0, 140.0)
        named_attribute_006.location = (540.0, -80.0)
        math_002.location = (820.0, 40.0)
        scene_time.location = (640.0, -20.0)
        boolean_math.location = (1160.0, 0.0)
        boolean_math_001.location = (980.0, 40.0)
        group_input.location = (460.0, 100.0)
        group_output.location = (1540.0, 100.0)

        # Set dimensions
        delete_geometry.width, delete_geometry.height = 140.0, 100.0
        named_attribute_006.width, named_attribute_006.height = 240.0, 100.0
        math_002.width, math_002.height = 140.0, 100.0
        scene_time.width, scene_time.height = 140.0, 100.0
        boolean_math.width, boolean_math.height = 140.0, 100.0
        boolean_math_001.width, boolean_math_001.height = 140.0, 100.0
        group_input.width, group_input.height = 140.0, 100.0
        group_output.width, group_output.height = 140.0, 100.0

        # initialize blended_mpm_remove_broken links
        # boolean_math_001.Boolean -> boolean_math.Boolean
        blended_mpm_remove_broken.links.new(
            boolean_math_001.outputs[0], boolean_math.inputs[0]
        )
        # scene_time.Frame -> math_002.Value
        blended_mpm_remove_broken.links.new(scene_time.outputs[1], math_002.inputs[0])
        # named_attribute_006.Attribute -> math_002.Value
        blended_mpm_remove_broken.links.new(
            named_attribute_006.outputs[0], math_002.inputs[1]
        )
        # math_002.Value -> boolean_math_001.Boolean
        blended_mpm_remove_broken.links.new(
            math_002.outputs[0], boolean_math_001.inputs[0]
        )
        # named_attribute_006.Exists -> boolean_math.Boolean
        blended_mpm_remove_broken.links.new(
            named_attribute_006.outputs[1], boolean_math.inputs[1]
        )
        # boolean_math.Boolean -> delete_geometry.Selection
        blended_mpm_remove_broken.links.new(
            boolean_math.outputs[0], delete_geometry.inputs[1]
        )
        # group_input.Geometry -> delete_geometry.Geometry
        blended_mpm_remove_broken.links.new(
            group_input.outputs[0], delete_geometry.inputs[0]
        )
        # delete_geometry.Geometry -> group_output.Geometry
        blended_mpm_remove_broken.links.new(
            delete_geometry.outputs[0], group_output.inputs[0]
        )
        return blended_mpm_remove_broken

    return blended_mpm_remove_broken_node_group()
