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


def create_geometry_nodes_restrict_view():
    # initialize blended_mpm_falls_outside_of node group
    def blended_mpm_falls_outside_of_node_group():
        blended_mpm_falls_outside_of = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Falls Outside Of"
        )

        blended_mpm_falls_outside_of.color_tag = "NONE"
        blended_mpm_falls_outside_of.description = ""
        blended_mpm_falls_outside_of.default_group_node_width = 140

        # blended_mpm_falls_outside_of interface
        # Socket Boolean
        boolean_socket = blended_mpm_falls_outside_of.interface.new_socket(
            name="Boolean", in_out="OUTPUT", socket_type="NodeSocketBool"
        )
        boolean_socket.default_value = False
        boolean_socket.attribute_domain = "POINT"

        # Socket Value
        value_socket = blended_mpm_falls_outside_of.interface.new_socket(
            name="Value", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        value_socket.default_value = 0.5
        value_socket.min_value = -10000.0
        value_socket.max_value = 10000.0
        value_socket.subtype = "NONE"
        value_socket.attribute_domain = "POINT"

        # Socket Max
        max_socket = blended_mpm_falls_outside_of.interface.new_socket(
            name="Max", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        max_socket.default_value = 1.0
        max_socket.min_value = -10000.0
        max_socket.max_value = 10000.0
        max_socket.subtype = "NONE"
        max_socket.attribute_domain = "POINT"

        # Socket Min
        min_socket = blended_mpm_falls_outside_of.interface.new_socket(
            name="Min", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        min_socket.default_value = -1.0
        min_socket.min_value = -10000.0
        min_socket.max_value = 10000.0
        min_socket.subtype = "NONE"
        min_socket.attribute_domain = "POINT"

        # initialize blended_mpm_falls_outside_of nodes
        # node Group Output
        group_output = blended_mpm_falls_outside_of.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # node Group Input
        group_input = blended_mpm_falls_outside_of.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

        # node Math
        math = blended_mpm_falls_outside_of.nodes.new("ShaderNodeMath")
        math.name = "Math"
        math.operation = "GREATER_THAN"
        math.use_clamp = False

        # node Math.001
        math_001 = blended_mpm_falls_outside_of.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.operation = "LESS_THAN"
        math_001.use_clamp = False

        # node Boolean Math
        boolean_math = blended_mpm_falls_outside_of.nodes.new("FunctionNodeBooleanMath")
        boolean_math.name = "Boolean Math"
        boolean_math.operation = "OR"

        # Set locations
        group_output.location = (300.0, 0.0)
        group_input.location = (-340.0, 0.0)
        math.location = (-90.0, 80.0)
        math_001.location = (-90.0, -80.0)
        boolean_math.location = (140.0, 0.0)

        # Set dimensions
        group_output.width, group_output.height = 140.0, 100.0
        group_input.width, group_input.height = 140.0, 100.0
        math.width, math.height = 140.0, 100.0
        math_001.width, math_001.height = 140.0, 100.0
        boolean_math.width, boolean_math.height = 140.0, 100.0

        # initialize blended_mpm_falls_outside_of links
        # math.Value -> boolean_math.Boolean
        blended_mpm_falls_outside_of.links.new(math.outputs[0], boolean_math.inputs[0])
        # math_001.Value -> boolean_math.Boolean
        blended_mpm_falls_outside_of.links.new(
            math_001.outputs[0], boolean_math.inputs[1]
        )
        # group_input.Value -> math.Value
        blended_mpm_falls_outside_of.links.new(group_input.outputs[0], math.inputs[0])
        # group_input.Value -> math_001.Value
        blended_mpm_falls_outside_of.links.new(
            group_input.outputs[0], math_001.inputs[0]
        )
        # boolean_math.Boolean -> group_output.Boolean
        blended_mpm_falls_outside_of.links.new(
            boolean_math.outputs[0], group_output.inputs[0]
        )
        # group_input.Max -> math.Value
        blended_mpm_falls_outside_of.links.new(group_input.outputs[1], math.inputs[1])
        # group_input.Min -> math_001.Value
        blended_mpm_falls_outside_of.links.new(
            group_input.outputs[2], math_001.inputs[1]
        )
        return blended_mpm_falls_outside_of

    blended_mpm_falls_outside_of = blended_mpm_falls_outside_of_node_group()

    # initialize blended_mpm_restrict_view node group
    def blended_mpm_restrict_view_node_group():
        blended_mpm_restrict_view = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Restrict View"
        )

        blended_mpm_restrict_view.color_tag = "NONE"
        blended_mpm_restrict_view.description = ""
        blended_mpm_restrict_view.default_group_node_width = 140

        blended_mpm_restrict_view.is_modifier = True

        # blended_mpm_restrict_view interface
        # Socket Geometry
        geometry_socket = blended_mpm_restrict_view.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = blended_mpm_restrict_view.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Object
        object_socket = blended_mpm_restrict_view.interface.new_socket(
            name="Object", in_out="INPUT", socket_type="NodeSocketObject"
        )
        object_socket.attribute_domain = "POINT"

        # initialize blended_mpm_restrict_view nodes
        # node Group Input
        group_input_1 = blended_mpm_restrict_view.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Group Output
        group_output_1 = blended_mpm_restrict_view.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Object Info
        object_info = blended_mpm_restrict_view.nodes.new("GeometryNodeObjectInfo")
        object_info.name = "Object Info"
        object_info.transform_space = "ORIGINAL"
        # As Instance
        object_info.inputs[1].default_value = False

        # node Invert Matrix
        invert_matrix = blended_mpm_restrict_view.nodes.new("FunctionNodeInvertMatrix")
        invert_matrix.name = "Invert Matrix"

        # node Position
        position = blended_mpm_restrict_view.nodes.new("GeometryNodeInputPosition")
        position.name = "Position"

        # node Transform Point
        transform_point = blended_mpm_restrict_view.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point.name = "Transform Point"

        # node Separate XYZ
        separate_xyz = blended_mpm_restrict_view.nodes.new("ShaderNodeSeparateXYZ")
        separate_xyz.name = "Separate XYZ"

        # node Boolean Math.001
        boolean_math_001 = blended_mpm_restrict_view.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_001.name = "Boolean Math.001"
        boolean_math_001.operation = "OR"

        # node Boolean Math.003
        boolean_math_003 = blended_mpm_restrict_view.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_003.name = "Boolean Math.003"
        boolean_math_003.operation = "OR"

        # node Delete Geometry
        delete_geometry = blended_mpm_restrict_view.nodes.new(
            "GeometryNodeDeleteGeometry"
        )
        delete_geometry.name = "Delete Geometry"
        delete_geometry.domain = "POINT"
        delete_geometry.mode = "ALL"

        # node Group
        group = blended_mpm_restrict_view.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = blended_mpm_falls_outside_of
        # Socket_2
        group.inputs[1].default_value = 1.0
        # Socket_3
        group.inputs[2].default_value = -1.0

        # node Group.001
        group_001 = blended_mpm_restrict_view.nodes.new("GeometryNodeGroup")
        group_001.name = "Group.001"
        group_001.node_tree = blended_mpm_falls_outside_of
        # Socket_2
        group_001.inputs[1].default_value = 1.0
        # Socket_3
        group_001.inputs[2].default_value = -1.0

        # node Group.002
        group_002 = blended_mpm_restrict_view.nodes.new("GeometryNodeGroup")
        group_002.name = "Group.002"
        group_002.node_tree = blended_mpm_falls_outside_of
        # Socket_2
        group_002.inputs[1].default_value = 1.0
        # Socket_3
        group_002.inputs[2].default_value = -1.0

        # node Reroute
        reroute = blended_mpm_restrict_view.nodes.new("NodeReroute")
        reroute.name = "Reroute"
        reroute.socket_idname = "NodeSocketGeometry"

        # Set locations
        group_input_1.location = (-540.0, -140.0)
        group_output_1.location = (1340.0, 100.0)
        object_info.location = (-360.0, -180.0)
        invert_matrix.location = (-180.0, -220.0)
        position.location = (-180.0, -140.0)
        transform_point.location = (0.0, -180.0)
        separate_xyz.location = (160.0, -180.0)
        boolean_math_001.location = (720.0, 0.0)
        boolean_math_003.location = (900.0, 0.0)
        delete_geometry.location = (1160.0, 140.0)
        group.location = (380.0, 0.0)
        group_001.location = (380.0, -160.0)
        group_002.location = (380.0, -320.0)
        reroute.location = (-360.0, 20.0)

        # Set dimensions
        group_input_1.width, group_input_1.height = 140.0, 100.0
        group_output_1.width, group_output_1.height = 140.0, 100.0
        object_info.width, object_info.height = 140.0, 100.0
        invert_matrix.width, invert_matrix.height = 140.0, 100.0
        position.width, position.height = 140.0, 100.0
        transform_point.width, transform_point.height = 140.0, 100.0
        separate_xyz.width, separate_xyz.height = 140.0, 100.0
        boolean_math_001.width, boolean_math_001.height = 140.0, 100.0
        boolean_math_003.width, boolean_math_003.height = 140.0, 100.0
        delete_geometry.width, delete_geometry.height = 140.0, 100.0
        group.width, group.height = 280.0, 100.0
        group_001.width, group_001.height = 280.0, 100.0
        group_002.width, group_002.height = 280.0, 100.0
        reroute.width, reroute.height = 10.0, 100.0

        # initialize blended_mpm_restrict_view links
        # delete_geometry.Geometry -> group_output_1.Geometry
        blended_mpm_restrict_view.links.new(
            delete_geometry.outputs[0], group_output_1.inputs[0]
        )
        # group_input_1.Object -> object_info.Object
        blended_mpm_restrict_view.links.new(
            group_input_1.outputs[1], object_info.inputs[0]
        )
        # object_info.Transform -> invert_matrix.Matrix
        blended_mpm_restrict_view.links.new(
            object_info.outputs[0], invert_matrix.inputs[0]
        )
        # position.Position -> transform_point.Vector
        blended_mpm_restrict_view.links.new(
            position.outputs[0], transform_point.inputs[0]
        )
        # invert_matrix.Matrix -> transform_point.Transform
        blended_mpm_restrict_view.links.new(
            invert_matrix.outputs[0], transform_point.inputs[1]
        )
        # transform_point.Vector -> separate_xyz.Vector
        blended_mpm_restrict_view.links.new(
            transform_point.outputs[0], separate_xyz.inputs[0]
        )
        # reroute.Output -> delete_geometry.Geometry
        blended_mpm_restrict_view.links.new(
            reroute.outputs[0], delete_geometry.inputs[0]
        )
        # separate_xyz.X -> group.Value
        blended_mpm_restrict_view.links.new(separate_xyz.outputs[0], group.inputs[0])
        # separate_xyz.Y -> group_001.Value
        blended_mpm_restrict_view.links.new(
            separate_xyz.outputs[1], group_001.inputs[0]
        )
        # separate_xyz.Z -> group_002.Value
        blended_mpm_restrict_view.links.new(
            separate_xyz.outputs[2], group_002.inputs[0]
        )
        # group.Boolean -> boolean_math_001.Boolean
        blended_mpm_restrict_view.links.new(
            group.outputs[0], boolean_math_001.inputs[0]
        )
        # group_001.Boolean -> boolean_math_001.Boolean
        blended_mpm_restrict_view.links.new(
            group_001.outputs[0], boolean_math_001.inputs[1]
        )
        # group_002.Boolean -> boolean_math_003.Boolean
        blended_mpm_restrict_view.links.new(
            group_002.outputs[0], boolean_math_003.inputs[1]
        )
        # boolean_math_001.Boolean -> boolean_math_003.Boolean
        blended_mpm_restrict_view.links.new(
            boolean_math_001.outputs[0], boolean_math_003.inputs[0]
        )
        # boolean_math_003.Boolean -> delete_geometry.Selection
        blended_mpm_restrict_view.links.new(
            boolean_math_003.outputs[0], delete_geometry.inputs[1]
        )
        # group_input_1.Geometry -> reroute.Input
        blended_mpm_restrict_view.links.new(group_input_1.outputs[0], reroute.inputs[0])
        return blended_mpm_restrict_view

    return blended_mpm_restrict_view_node_group()
