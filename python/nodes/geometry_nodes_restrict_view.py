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


def create_geometry_nodes_restrict_view():
    # initialize squishy_volumes_falls_outside_of node group
    def squishy_volumes_falls_outside_of_node_group():
        squishy_volumes_falls_outside_of = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Falls Outside Of"
        )

        squishy_volumes_falls_outside_of.color_tag = "NONE"
        squishy_volumes_falls_outside_of.description = ""
        squishy_volumes_falls_outside_of.default_group_node_width = 140

        # squishy_volumes_falls_outside_of interface
        # Socket Boolean
        boolean_socket = squishy_volumes_falls_outside_of.interface.new_socket(
            name="Boolean", in_out="OUTPUT", socket_type="NodeSocketBool"
        )
        boolean_socket.attribute_domain = "POINT"

        # Socket Value
        value_socket = squishy_volumes_falls_outside_of.interface.new_socket(
            name="Value", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        value_socket.subtype = "NONE"
        value_socket.attribute_domain = "POINT"

        # Socket Max
        max_socket = squishy_volumes_falls_outside_of.interface.new_socket(
            name="Max", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        max_socket.subtype = "NONE"
        max_socket.attribute_domain = "POINT"

        # Socket Min
        min_socket = squishy_volumes_falls_outside_of.interface.new_socket(
            name="Min", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        min_socket.subtype = "NONE"
        min_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_falls_outside_of nodes
        # node Group Output
        group_output = squishy_volumes_falls_outside_of.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # node Group Input
        group_input = squishy_volumes_falls_outside_of.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

        # node Math
        math = squishy_volumes_falls_outside_of.nodes.new("ShaderNodeMath")
        math.name = "Math"
        math.operation = "GREATER_THAN"
        math.use_clamp = False

        # node Math.001
        math_001 = squishy_volumes_falls_outside_of.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.operation = "LESS_THAN"
        math_001.use_clamp = False

        # node Boolean Math
        boolean_math = squishy_volumes_falls_outside_of.nodes.new(
            "FunctionNodeBooleanMath"
        )
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

        # initialize squishy_volumes_falls_outside_of links
        # math.Value -> boolean_math.Boolean
        squishy_volumes_falls_outside_of.links.new(
            math.outputs[0], boolean_math.inputs[0]
        )
        # math_001.Value -> boolean_math.Boolean
        squishy_volumes_falls_outside_of.links.new(
            math_001.outputs[0], boolean_math.inputs[1]
        )
        # group_input.Value -> math.Value
        squishy_volumes_falls_outside_of.links.new(
            group_input.outputs[0], math.inputs[0]
        )
        # group_input.Value -> math_001.Value
        squishy_volumes_falls_outside_of.links.new(
            group_input.outputs[0], math_001.inputs[0]
        )
        # boolean_math.Boolean -> group_output.Boolean
        squishy_volumes_falls_outside_of.links.new(
            boolean_math.outputs[0], group_output.inputs[0]
        )
        # group_input.Max -> math.Value
        squishy_volumes_falls_outside_of.links.new(
            group_input.outputs[1], math.inputs[1]
        )
        # group_input.Min -> math_001.Value
        squishy_volumes_falls_outside_of.links.new(
            group_input.outputs[2], math_001.inputs[1]
        )
        return squishy_volumes_falls_outside_of

    squishy_volumes_falls_outside_of = squishy_volumes_falls_outside_of_node_group()

    # initialize squishy_volumes_change_of_basis node group
    def squishy_volumes_change_of_basis_node_group():
        squishy_volumes_change_of_basis = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Change of Basis"
        )

        squishy_volumes_change_of_basis.color_tag = "NONE"
        squishy_volumes_change_of_basis.description = ""
        squishy_volumes_change_of_basis.default_group_node_width = 140

        # squishy_volumes_change_of_basis interface
        # Socket Transform
        transform_socket = squishy_volumes_change_of_basis.interface.new_socket(
            name="Transform", in_out="OUTPUT", socket_type="NodeSocketMatrix"
        )
        transform_socket.attribute_domain = "POINT"

        # Socket Target Space Obj
        target_space_obj_socket = squishy_volumes_change_of_basis.interface.new_socket(
            name="Target Space Obj", in_out="INPUT", socket_type="NodeSocketObject"
        )
        target_space_obj_socket.attribute_domain = "POINT"

        # Socket Source Space Obj
        source_space_obj_socket = squishy_volumes_change_of_basis.interface.new_socket(
            name="Source Space Obj", in_out="INPUT", socket_type="NodeSocketObject"
        )
        source_space_obj_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_change_of_basis nodes
        # node Group Output
        group_output_1 = squishy_volumes_change_of_basis.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Group Input
        group_input_1 = squishy_volumes_change_of_basis.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Object Info.002
        object_info_002 = squishy_volumes_change_of_basis.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_002.name = "Object Info.002"
        object_info_002.transform_space = "ORIGINAL"
        # As Instance
        object_info_002.inputs[1].default_value = False

        # node Object Info.005
        object_info_005 = squishy_volumes_change_of_basis.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_005.name = "Object Info.005"
        object_info_005.transform_space = "ORIGINAL"
        # As Instance
        object_info_005.inputs[1].default_value = False

        # node Multiply Matrices.003
        multiply_matrices_003 = squishy_volumes_change_of_basis.nodes.new(
            "FunctionNodeMatrixMultiply"
        )
        multiply_matrices_003.name = "Multiply Matrices.003"

        # node Invert Matrix.002
        invert_matrix_002 = squishy_volumes_change_of_basis.nodes.new(
            "FunctionNodeInvertMatrix"
        )
        invert_matrix_002.name = "Invert Matrix.002"

        # Set locations
        group_output_1.location = (320.0, -60.0)
        group_input_1.location = (-520.0, -80.0)
        object_info_002.location = (-280.0, 120.0)
        object_info_005.location = (-280.0, -120.0)
        multiply_matrices_003.location = (140.0, -60.0)
        invert_matrix_002.location = (-80.0, 20.0)

        # Set dimensions
        group_output_1.width, group_output_1.height = 140.0, 100.0
        group_input_1.width, group_input_1.height = 140.0, 100.0
        object_info_002.width, object_info_002.height = 140.0, 100.0
        object_info_005.width, object_info_005.height = 140.0, 100.0
        multiply_matrices_003.width, multiply_matrices_003.height = 140.0, 100.0
        invert_matrix_002.width, invert_matrix_002.height = 140.0, 100.0

        # initialize squishy_volumes_change_of_basis links
        # invert_matrix_002.Matrix -> multiply_matrices_003.Matrix
        squishy_volumes_change_of_basis.links.new(
            invert_matrix_002.outputs[0], multiply_matrices_003.inputs[0]
        )
        # object_info_002.Transform -> invert_matrix_002.Matrix
        squishy_volumes_change_of_basis.links.new(
            object_info_002.outputs[0], invert_matrix_002.inputs[0]
        )
        # object_info_005.Transform -> multiply_matrices_003.Matrix
        squishy_volumes_change_of_basis.links.new(
            object_info_005.outputs[0], multiply_matrices_003.inputs[1]
        )
        # group_input_1.Target Space Obj -> object_info_002.Object
        squishy_volumes_change_of_basis.links.new(
            group_input_1.outputs[0], object_info_002.inputs[0]
        )
        # group_input_1.Source Space Obj -> object_info_005.Object
        squishy_volumes_change_of_basis.links.new(
            group_input_1.outputs[1], object_info_005.inputs[0]
        )
        # multiply_matrices_003.Matrix -> group_output_1.Transform
        squishy_volumes_change_of_basis.links.new(
            multiply_matrices_003.outputs[0], group_output_1.inputs[0]
        )
        return squishy_volumes_change_of_basis

    squishy_volumes_change_of_basis = squishy_volumes_change_of_basis_node_group()

    # initialize squishy_volumes_restrict_view node group
    def squishy_volumes_restrict_view_node_group():
        squishy_volumes_restrict_view = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Restrict View"
        )

        squishy_volumes_restrict_view.color_tag = "NONE"
        squishy_volumes_restrict_view.description = ""
        squishy_volumes_restrict_view.default_group_node_width = 140

        squishy_volumes_restrict_view.is_modifier = True

        # squishy_volumes_restrict_view interface
        # Socket Geometry
        geometry_socket = squishy_volumes_restrict_view.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = squishy_volumes_restrict_view.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Object
        object_socket = squishy_volumes_restrict_view.interface.new_socket(
            name="Object", in_out="INPUT", socket_type="NodeSocketObject"
        )
        object_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_restrict_view nodes
        # node Group Input
        group_input_2 = squishy_volumes_restrict_view.nodes.new("NodeGroupInput")
        group_input_2.name = "Group Input"

        # node Group Output
        group_output_2 = squishy_volumes_restrict_view.nodes.new("NodeGroupOutput")
        group_output_2.name = "Group Output"
        group_output_2.is_active_output = True

        # node Position
        position = squishy_volumes_restrict_view.nodes.new("GeometryNodeInputPosition")
        position.name = "Position"

        # node Transform Point
        transform_point = squishy_volumes_restrict_view.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point.name = "Transform Point"

        # node Separate XYZ
        separate_xyz = squishy_volumes_restrict_view.nodes.new("ShaderNodeSeparateXYZ")
        separate_xyz.name = "Separate XYZ"

        # node Boolean Math.001
        boolean_math_001 = squishy_volumes_restrict_view.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_001.name = "Boolean Math.001"
        boolean_math_001.operation = "OR"

        # node Boolean Math.003
        boolean_math_003 = squishy_volumes_restrict_view.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_003.name = "Boolean Math.003"
        boolean_math_003.operation = "OR"

        # node Delete Geometry
        delete_geometry = squishy_volumes_restrict_view.nodes.new(
            "GeometryNodeDeleteGeometry"
        )
        delete_geometry.name = "Delete Geometry"
        delete_geometry.domain = "POINT"
        delete_geometry.mode = "ALL"

        # node Group
        group = squishy_volumes_restrict_view.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = squishy_volumes_falls_outside_of
        # Socket_2
        group.inputs[1].default_value = 1.0
        # Socket_3
        group.inputs[2].default_value = -1.0

        # node Group.001
        group_001 = squishy_volumes_restrict_view.nodes.new("GeometryNodeGroup")
        group_001.name = "Group.001"
        group_001.node_tree = squishy_volumes_falls_outside_of
        # Socket_2
        group_001.inputs[1].default_value = 1.0
        # Socket_3
        group_001.inputs[2].default_value = -1.0

        # node Group.002
        group_002 = squishy_volumes_restrict_view.nodes.new("GeometryNodeGroup")
        group_002.name = "Group.002"
        group_002.node_tree = squishy_volumes_falls_outside_of
        # Socket_2
        group_002.inputs[1].default_value = 1.0
        # Socket_3
        group_002.inputs[2].default_value = -1.0

        # node Squishy Volumes Change of Basis
        squishy_volumes_change_of_basis_1 = squishy_volumes_restrict_view.nodes.new(
            "GeometryNodeGroup"
        )
        squishy_volumes_change_of_basis_1.name = "Squishy Volumes Change of Basis"
        squishy_volumes_change_of_basis_1.node_tree = squishy_volumes_change_of_basis

        # node Self Object
        self_object = squishy_volumes_restrict_view.nodes.new("GeometryNodeSelfObject")
        self_object.name = "Self Object"

        # Set locations
        group_input_2.location = (-540.0, 80.0)
        group_output_2.location = (1260.0, 80.0)
        position.location = (-180.0, -140.0)
        transform_point.location = (0.0, -180.0)
        separate_xyz.location = (160.0, -180.0)
        boolean_math_001.location = (720.0, 0.0)
        boolean_math_003.location = (900.0, 0.0)
        delete_geometry.location = (1080.0, 120.0)
        group.location = (380.0, 0.0)
        group_001.location = (380.0, -160.0)
        group_002.location = (380.0, -320.0)
        squishy_volumes_change_of_basis_1.location = (-320.0, -220.0)
        self_object.location = (-520.0, -280.0)

        # Set dimensions
        group_input_2.width, group_input_2.height = 140.0, 100.0
        group_output_2.width, group_output_2.height = 140.0, 100.0
        position.width, position.height = 140.0, 100.0
        transform_point.width, transform_point.height = 140.0, 100.0
        separate_xyz.width, separate_xyz.height = 140.0, 100.0
        boolean_math_001.width, boolean_math_001.height = 140.0, 100.0
        boolean_math_003.width, boolean_math_003.height = 140.0, 100.0
        delete_geometry.width, delete_geometry.height = 140.0, 100.0
        group.width, group.height = 280.0, 100.0
        group_001.width, group_001.height = 280.0, 100.0
        group_002.width, group_002.height = 280.0, 100.0
        (
            squishy_volumes_change_of_basis_1.width,
            squishy_volumes_change_of_basis_1.height,
        ) = (
            280.0,
            100.0,
        )
        self_object.width, self_object.height = 140.0, 100.0

        # initialize squishy_volumes_restrict_view links
        # delete_geometry.Geometry -> group_output_2.Geometry
        squishy_volumes_restrict_view.links.new(
            delete_geometry.outputs[0], group_output_2.inputs[0]
        )
        # position.Position -> transform_point.Vector
        squishy_volumes_restrict_view.links.new(
            position.outputs[0], transform_point.inputs[0]
        )
        # transform_point.Vector -> separate_xyz.Vector
        squishy_volumes_restrict_view.links.new(
            transform_point.outputs[0], separate_xyz.inputs[0]
        )
        # group_input_2.Geometry -> delete_geometry.Geometry
        squishy_volumes_restrict_view.links.new(
            group_input_2.outputs[0], delete_geometry.inputs[0]
        )
        # separate_xyz.X -> group.Value
        squishy_volumes_restrict_view.links.new(
            separate_xyz.outputs[0], group.inputs[0]
        )
        # separate_xyz.Y -> group_001.Value
        squishy_volumes_restrict_view.links.new(
            separate_xyz.outputs[1], group_001.inputs[0]
        )
        # separate_xyz.Z -> group_002.Value
        squishy_volumes_restrict_view.links.new(
            separate_xyz.outputs[2], group_002.inputs[0]
        )
        # group.Boolean -> boolean_math_001.Boolean
        squishy_volumes_restrict_view.links.new(
            group.outputs[0], boolean_math_001.inputs[0]
        )
        # group_001.Boolean -> boolean_math_001.Boolean
        squishy_volumes_restrict_view.links.new(
            group_001.outputs[0], boolean_math_001.inputs[1]
        )
        # group_002.Boolean -> boolean_math_003.Boolean
        squishy_volumes_restrict_view.links.new(
            group_002.outputs[0], boolean_math_003.inputs[1]
        )
        # boolean_math_001.Boolean -> boolean_math_003.Boolean
        squishy_volumes_restrict_view.links.new(
            boolean_math_001.outputs[0], boolean_math_003.inputs[0]
        )
        # boolean_math_003.Boolean -> delete_geometry.Selection
        squishy_volumes_restrict_view.links.new(
            boolean_math_003.outputs[0], delete_geometry.inputs[1]
        )
        # squishy_volumes_change_of_basis_1.Transform -> transform_point.Transform
        squishy_volumes_restrict_view.links.new(
            squishy_volumes_change_of_basis_1.outputs[0], transform_point.inputs[1]
        )
        # group_input_2.Object -> squishy_volumes_change_of_basis_1.Target Space Obj
        squishy_volumes_restrict_view.links.new(
            group_input_2.outputs[1], squishy_volumes_change_of_basis_1.inputs[0]
        )
        # self_object.Self Object -> squishy_volumes_change_of_basis_1.Source Space Obj
        squishy_volumes_restrict_view.links.new(
            self_object.outputs[0], squishy_volumes_change_of_basis_1.inputs[1]
        )
        return squishy_volumes_restrict_view

    return squishy_volumes_restrict_view_node_group()
