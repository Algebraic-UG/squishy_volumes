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

from ..magic_consts import (
    SQUISHY_VOLUMES_REFERENCE_INDEX,
    SQUISHY_VOLUMES_REFERENCE_OFFSET,
    SQUISHY_VOLUMES_TRANSFORM,
)


def create_geometry_nodes_move_with_reference():
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
        group_output = squishy_volumes_change_of_basis.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # node Group Input
        group_input = squishy_volumes_change_of_basis.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

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
        group_output.location = (320.0, -60.0)
        group_input.location = (-520.0, -80.0)
        object_info_002.location = (-280.0, 120.0)
        object_info_005.location = (-280.0, -120.0)
        multiply_matrices_003.location = (140.0, -60.0)
        invert_matrix_002.location = (-80.0, 20.0)

        # Set dimensions
        group_output.width, group_output.height = 140.0, 100.0
        group_input.width, group_input.height = 140.0, 100.0
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
        # group_input.Target Space Obj -> object_info_002.Object
        squishy_volumes_change_of_basis.links.new(
            group_input.outputs[0], object_info_002.inputs[0]
        )
        # group_input.Source Space Obj -> object_info_005.Object
        squishy_volumes_change_of_basis.links.new(
            group_input.outputs[1], object_info_005.inputs[0]
        )
        # multiply_matrices_003.Matrix -> group_output.Transform
        squishy_volumes_change_of_basis.links.new(
            multiply_matrices_003.outputs[0], group_output.inputs[0]
        )
        return squishy_volumes_change_of_basis

    squishy_volumes_change_of_basis = squishy_volumes_change_of_basis_node_group()

    # initialize squishy_volumes_move_with_reference node group
    def squishy_volumes_move_with_reference_node_group():
        squishy_volumes_move_with_reference = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Move With Reference"
        )

        squishy_volumes_move_with_reference.color_tag = "NONE"
        squishy_volumes_move_with_reference.description = ""
        squishy_volumes_move_with_reference.default_group_node_width = 140

        squishy_volumes_move_with_reference.is_modifier = True

        # squishy_volumes_move_with_reference interface
        # Socket Geometry
        geometry_socket = squishy_volumes_move_with_reference.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = squishy_volumes_move_with_reference.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Squishy Volumes Particles
        squishy_volumes_particles_socket = (
            squishy_volumes_move_with_reference.interface.new_socket(
                name="Squishy Volumes Particles",
                in_out="INPUT",
                socket_type="NodeSocketObject",
            )
        )
        squishy_volumes_particles_socket.attribute_domain = "POINT"

        # Socket Visible Transform
        visible_transform_socket = (
            squishy_volumes_move_with_reference.interface.new_socket(
                name="Visible Transform", in_out="INPUT", socket_type="NodeSocketMenu"
            )
        )
        visible_transform_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_move_with_reference nodes
        # node Group Output
        group_output_1 = squishy_volumes_move_with_reference.nodes.new(
            "NodeGroupOutput"
        )
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Set Position
        set_position = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeSetPosition"
        )
        set_position.name = "Set Position"
        # Selection
        set_position.inputs[1].default_value = True
        # Offset
        set_position.inputs[3].default_value = (0.0, 0.0, 0.0)

        # node Named Attribute
        named_attribute = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "INT"
        # Name
        named_attribute.inputs[0].default_value = SQUISHY_VOLUMES_REFERENCE_INDEX

        # node Named Attribute.002
        named_attribute_002 = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_002.name = "Named Attribute.002"
        named_attribute_002.data_type = "FLOAT_VECTOR"
        # Name
        named_attribute_002.inputs[0].default_value = SQUISHY_VOLUMES_REFERENCE_OFFSET

        # node Object Info
        object_info = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info.name = "Object Info"
        object_info.transform_space = "ORIGINAL"
        # As Instance
        object_info.inputs[1].default_value = False

        # node Sample Index
        sample_index = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index.name = "Sample Index"
        sample_index.clamp = False
        sample_index.data_type = "FLOAT4X4"
        sample_index.domain = "POINT"

        # node Named Attribute.001
        named_attribute_001 = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_001.name = "Named Attribute.001"
        named_attribute_001.data_type = "FLOAT4X4"
        # Name
        named_attribute_001.inputs[0].default_value = SQUISHY_VOLUMES_TRANSFORM

        # node Transform Point
        transform_point = squishy_volumes_move_with_reference.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point.name = "Transform Point"

        # node Self Object
        self_object = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeSelfObject"
        )
        self_object.name = "Self Object"

        # node Transform Point.001
        transform_point_001 = squishy_volumes_move_with_reference.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point_001.name = "Transform Point.001"

        # node Squishy Volumes Change of Basis.001
        squishy_volumes_change_of_basis_001 = (
            squishy_volumes_move_with_reference.nodes.new("GeometryNodeGroup")
        )
        squishy_volumes_change_of_basis_001.name = "Squishy Volumes Change of Basis.001"
        squishy_volumes_change_of_basis_001.node_tree = squishy_volumes_change_of_basis

        # node Group Input
        group_input_1 = squishy_volumes_move_with_reference.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Group Input.001
        group_input_001 = squishy_volumes_move_with_reference.nodes.new(
            "NodeGroupInput"
        )
        group_input_001.name = "Group Input.001"

        # node Group Input.002
        group_input_002 = squishy_volumes_move_with_reference.nodes.new(
            "NodeGroupInput"
        )
        group_input_002.name = "Group Input.002"

        # node Menu Switch
        menu_switch = squishy_volumes_move_with_reference.nodes.new(
            "GeometryNodeMenuSwitch"
        )
        menu_switch.name = "Menu Switch"
        menu_switch.active_index = 1
        menu_switch.data_type = "VECTOR"
        menu_switch.enum_items.clear()
        menu_switch.enum_items.new("Self Object")
        menu_switch.enum_items[0].description = ""
        menu_switch.enum_items.new("Particle Object")
        menu_switch.enum_items[1].description = ""

        # Set locations
        group_output_1.location = (760.0, 100.0)
        set_position.location = (580.0, 100.0)
        named_attribute.location = (-900.0, -320.0)
        named_attribute_002.location = (-580.0, 20.0)
        object_info.location = (-860.0, 60.0)
        sample_index.location = (-480.0, -140.0)
        named_attribute_001.location = (-900.0, -180.0)
        transform_point.location = (-280.0, -40.0)
        self_object.location = (-600.0, -400.0)
        transform_point_001.location = (40.0, -180.0)
        squishy_volumes_change_of_basis_001.location = (-400.0, -380.0)
        group_input_1.location = (-1080.0, -40.0)
        group_input_001.location = (-640.0, -480.0)
        group_input_002.location = (120.0, 100.0)
        menu_switch.location = (360.0, 20.0)

        # Set dimensions
        group_output_1.width, group_output_1.height = 140.0, 100.0
        set_position.width, set_position.height = 140.0, 100.0
        named_attribute.width, named_attribute.height = 240.0, 100.0
        named_attribute_002.width, named_attribute_002.height = 240.0, 100.0
        object_info.width, object_info.height = 140.0, 100.0
        sample_index.width, sample_index.height = 140.0, 100.0
        named_attribute_001.width, named_attribute_001.height = 240.0, 100.0
        transform_point.width, transform_point.height = 140.0, 100.0
        self_object.width, self_object.height = 140.0, 100.0
        transform_point_001.width, transform_point_001.height = 140.0, 100.0
        (
            squishy_volumes_change_of_basis_001.width,
            squishy_volumes_change_of_basis_001.height,
        ) = (
            300.0,
            100.0,
        )
        group_input_1.width, group_input_1.height = 180.0, 100.0
        group_input_001.width, group_input_001.height = 180.0, 100.0
        group_input_002.width, group_input_002.height = 180.0, 100.0
        menu_switch.width, menu_switch.height = 140.0, 100.0

        # initialize squishy_volumes_move_with_reference links
        # set_position.Geometry -> group_output_1.Geometry
        squishy_volumes_move_with_reference.links.new(
            set_position.outputs[0], group_output_1.inputs[0]
        )
        # sample_index.Value -> transform_point.Transform
        squishy_volumes_move_with_reference.links.new(
            sample_index.outputs[0], transform_point.inputs[1]
        )
        # named_attribute_001.Attribute -> sample_index.Value
        squishy_volumes_move_with_reference.links.new(
            named_attribute_001.outputs[0], sample_index.inputs[1]
        )
        # object_info.Geometry -> sample_index.Geometry
        squishy_volumes_move_with_reference.links.new(
            object_info.outputs[4], sample_index.inputs[0]
        )
        # named_attribute_002.Attribute -> transform_point.Vector
        squishy_volumes_move_with_reference.links.new(
            named_attribute_002.outputs[0], transform_point.inputs[0]
        )
        # self_object.Self Object -> squishy_volumes_change_of_basis_001.Target Space Obj
        squishy_volumes_move_with_reference.links.new(
            self_object.outputs[0], squishy_volumes_change_of_basis_001.inputs[0]
        )
        # squishy_volumes_change_of_basis_001.Transform -> transform_point_001.Transform
        squishy_volumes_move_with_reference.links.new(
            squishy_volumes_change_of_basis_001.outputs[0],
            transform_point_001.inputs[1],
        )
        # transform_point.Vector -> transform_point_001.Vector
        squishy_volumes_move_with_reference.links.new(
            transform_point.outputs[0], transform_point_001.inputs[0]
        )
        # named_attribute.Attribute -> sample_index.Index
        squishy_volumes_move_with_reference.links.new(
            named_attribute.outputs[0], sample_index.inputs[2]
        )
        # group_input_1.Squishy Volumes Particles -> object_info.Object
        squishy_volumes_move_with_reference.links.new(
            group_input_1.outputs[1], object_info.inputs[0]
        )
        # group_input_001.Squishy Volumes Particles -> squishy_volumes_change_of_basis_001.Source Space Obj
        squishy_volumes_move_with_reference.links.new(
            group_input_001.outputs[1], squishy_volumes_change_of_basis_001.inputs[1]
        )
        # group_input_002.Geometry -> set_position.Geometry
        squishy_volumes_move_with_reference.links.new(
            group_input_002.outputs[0], set_position.inputs[0]
        )
        # group_input_002.Visible Transform -> menu_switch.Menu
        squishy_volumes_move_with_reference.links.new(
            group_input_002.outputs[2], menu_switch.inputs[0]
        )
        # menu_switch.Output -> set_position.Position
        squishy_volumes_move_with_reference.links.new(
            menu_switch.outputs[0], set_position.inputs[2]
        )
        # transform_point.Vector -> menu_switch.Self Object
        squishy_volumes_move_with_reference.links.new(
            transform_point.outputs[0], menu_switch.inputs[1]
        )
        # transform_point_001.Vector -> menu_switch.Particle Object
        squishy_volumes_move_with_reference.links.new(
            transform_point_001.outputs[0], menu_switch.inputs[2]
        )
        return squishy_volumes_move_with_reference

    return squishy_volumes_move_with_reference_node_group()
