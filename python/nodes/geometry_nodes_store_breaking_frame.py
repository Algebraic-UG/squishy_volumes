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

from ..magic_consts import (
    BLENDED_MPM_BREAKING_FRAME,
    BLENDED_MPM_COLLIDER_INSIDE,
    BLENDED_MPM_INITIAL_LENGTH,
    BLENDED_MPM_REFERENCE_INDEX,
    BLENDED_MPM_REFERENCE_OFFSET,
    BLENDED_MPM_TRANSFORM,
)


def create_geometry_nodes_store_breaking_frame():
    # initialize blended_mpm_change_of_basis node group
    def blended_mpm_change_of_basis_node_group():
        blended_mpm_change_of_basis = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Change of Basis"
        )

        blended_mpm_change_of_basis.color_tag = "NONE"
        blended_mpm_change_of_basis.description = ""
        blended_mpm_change_of_basis.default_group_node_width = 140

        # blended_mpm_change_of_basis interface
        # Socket Transform
        transform_socket = blended_mpm_change_of_basis.interface.new_socket(
            name="Transform", in_out="OUTPUT", socket_type="NodeSocketMatrix"
        )
        transform_socket.attribute_domain = "POINT"

        # Socket Target Space Obj
        target_space_obj_socket = blended_mpm_change_of_basis.interface.new_socket(
            name="Target Space Obj", in_out="INPUT", socket_type="NodeSocketObject"
        )
        target_space_obj_socket.attribute_domain = "POINT"

        # Socket Source Space Obj
        source_space_obj_socket = blended_mpm_change_of_basis.interface.new_socket(
            name="Source Space Obj", in_out="INPUT", socket_type="NodeSocketObject"
        )
        source_space_obj_socket.attribute_domain = "POINT"

        # initialize blended_mpm_change_of_basis nodes
        # node Group Output
        group_output = blended_mpm_change_of_basis.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # node Group Input
        group_input = blended_mpm_change_of_basis.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

        # node Object Info.002
        object_info_002 = blended_mpm_change_of_basis.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_002.name = "Object Info.002"
        object_info_002.transform_space = "ORIGINAL"
        # As Instance
        object_info_002.inputs[1].default_value = False

        # node Object Info.005
        object_info_005 = blended_mpm_change_of_basis.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_005.name = "Object Info.005"
        object_info_005.transform_space = "ORIGINAL"
        # As Instance
        object_info_005.inputs[1].default_value = False

        # node Multiply Matrices.003
        multiply_matrices_003 = blended_mpm_change_of_basis.nodes.new(
            "FunctionNodeMatrixMultiply"
        )
        multiply_matrices_003.name = "Multiply Matrices.003"

        # node Invert Matrix.002
        invert_matrix_002 = blended_mpm_change_of_basis.nodes.new(
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

        # initialize blended_mpm_change_of_basis links
        # invert_matrix_002.Matrix -> multiply_matrices_003.Matrix
        blended_mpm_change_of_basis.links.new(
            invert_matrix_002.outputs[0], multiply_matrices_003.inputs[0]
        )
        # object_info_002.Transform -> invert_matrix_002.Matrix
        blended_mpm_change_of_basis.links.new(
            object_info_002.outputs[0], invert_matrix_002.inputs[0]
        )
        # object_info_005.Transform -> multiply_matrices_003.Matrix
        blended_mpm_change_of_basis.links.new(
            object_info_005.outputs[0], multiply_matrices_003.inputs[1]
        )
        # group_input.Target Space Obj -> object_info_002.Object
        blended_mpm_change_of_basis.links.new(
            group_input.outputs[0], object_info_002.inputs[0]
        )
        # group_input.Source Space Obj -> object_info_005.Object
        blended_mpm_change_of_basis.links.new(
            group_input.outputs[1], object_info_005.inputs[0]
        )
        # multiply_matrices_003.Matrix -> group_output.Transform
        blended_mpm_change_of_basis.links.new(
            multiply_matrices_003.outputs[0], group_output.inputs[0]
        )
        return blended_mpm_change_of_basis

    blended_mpm_change_of_basis = blended_mpm_change_of_basis_node_group()

    # initialize blended_mpm_move_with_reference node group
    def blended_mpm_move_with_reference_node_group():
        blended_mpm_move_with_reference = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Move With Reference"
        )

        blended_mpm_move_with_reference.color_tag = "NONE"
        blended_mpm_move_with_reference.description = ""
        blended_mpm_move_with_reference.default_group_node_width = 140

        blended_mpm_move_with_reference.is_modifier = True

        # blended_mpm_move_with_reference interface
        # Socket Geometry
        geometry_socket = blended_mpm_move_with_reference.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = blended_mpm_move_with_reference.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Blended MPM Particles
        blended_mpm_particles_socket = (
            blended_mpm_move_with_reference.interface.new_socket(
                name="Blended MPM Particles",
                in_out="INPUT",
                socket_type="NodeSocketObject",
            )
        )
        blended_mpm_particles_socket.attribute_domain = "POINT"

        # Socket Visible Transform
        visible_transform_socket = blended_mpm_move_with_reference.interface.new_socket(
            name="Visible Transform", in_out="INPUT", socket_type="NodeSocketMenu"
        )
        visible_transform_socket.attribute_domain = "POINT"

        # initialize blended_mpm_move_with_reference nodes
        # node Group Output
        group_output_1 = blended_mpm_move_with_reference.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Set Position
        set_position = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeSetPosition"
        )
        set_position.name = "Set Position"
        # Selection
        set_position.inputs[1].default_value = True
        # Offset
        set_position.inputs[3].default_value = (0.0, 0.0, 0.0)

        # node Named Attribute
        named_attribute = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "INT"
        # Name
        named_attribute.inputs[0].default_value = BLENDED_MPM_REFERENCE_INDEX

        # node Named Attribute.002
        named_attribute_002 = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_002.name = "Named Attribute.002"
        named_attribute_002.data_type = "FLOAT_VECTOR"
        # Name
        named_attribute_002.inputs[0].default_value = BLENDED_MPM_REFERENCE_OFFSET

        # node Object Info
        object_info = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info.name = "Object Info"
        object_info.transform_space = "ORIGINAL"
        # As Instance
        object_info.inputs[1].default_value = False

        # node Sample Index
        sample_index = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index.name = "Sample Index"
        sample_index.clamp = False
        sample_index.data_type = "FLOAT4X4"
        sample_index.domain = "POINT"

        # node Named Attribute.001
        named_attribute_001 = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_001.name = "Named Attribute.001"
        named_attribute_001.data_type = "FLOAT4X4"
        # Name
        named_attribute_001.inputs[0].default_value = BLENDED_MPM_TRANSFORM

        # node Transform Point
        transform_point = blended_mpm_move_with_reference.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point.name = "Transform Point"

        # node Self Object
        self_object = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeSelfObject"
        )
        self_object.name = "Self Object"

        # node Transform Point.001
        transform_point_001 = blended_mpm_move_with_reference.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point_001.name = "Transform Point.001"

        # node Blended MPM Change of Basis.001
        blended_mpm_change_of_basis_001 = blended_mpm_move_with_reference.nodes.new(
            "GeometryNodeGroup"
        )
        blended_mpm_change_of_basis_001.name = "Blended MPM Change of Basis.001"
        blended_mpm_change_of_basis_001.node_tree = blended_mpm_change_of_basis

        # node Group Input
        group_input_1 = blended_mpm_move_with_reference.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Group Input.001
        group_input_001 = blended_mpm_move_with_reference.nodes.new("NodeGroupInput")
        group_input_001.name = "Group Input.001"

        # node Group Input.002
        group_input_002 = blended_mpm_move_with_reference.nodes.new("NodeGroupInput")
        group_input_002.name = "Group Input.002"

        # node Menu Switch
        menu_switch = blended_mpm_move_with_reference.nodes.new(
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
        blended_mpm_change_of_basis_001.location = (-400.0, -380.0)
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
            blended_mpm_change_of_basis_001.width,
            blended_mpm_change_of_basis_001.height,
        ) = 300.0, 100.0
        group_input_1.width, group_input_1.height = 180.0, 100.0
        group_input_001.width, group_input_001.height = 180.0, 100.0
        group_input_002.width, group_input_002.height = 180.0, 100.0
        menu_switch.width, menu_switch.height = 140.0, 100.0

        # initialize blended_mpm_move_with_reference links
        # set_position.Geometry -> group_output_1.Geometry
        blended_mpm_move_with_reference.links.new(
            set_position.outputs[0], group_output_1.inputs[0]
        )
        # sample_index.Value -> transform_point.Transform
        blended_mpm_move_with_reference.links.new(
            sample_index.outputs[0], transform_point.inputs[1]
        )
        # named_attribute_001.Attribute -> sample_index.Value
        blended_mpm_move_with_reference.links.new(
            named_attribute_001.outputs[0], sample_index.inputs[1]
        )
        # object_info.Geometry -> sample_index.Geometry
        blended_mpm_move_with_reference.links.new(
            object_info.outputs[4], sample_index.inputs[0]
        )
        # named_attribute_002.Attribute -> transform_point.Vector
        blended_mpm_move_with_reference.links.new(
            named_attribute_002.outputs[0], transform_point.inputs[0]
        )
        # self_object.Self Object -> blended_mpm_change_of_basis_001.Target Space Obj
        blended_mpm_move_with_reference.links.new(
            self_object.outputs[0], blended_mpm_change_of_basis_001.inputs[0]
        )
        # blended_mpm_change_of_basis_001.Transform -> transform_point_001.Transform
        blended_mpm_move_with_reference.links.new(
            blended_mpm_change_of_basis_001.outputs[0], transform_point_001.inputs[1]
        )
        # transform_point.Vector -> transform_point_001.Vector
        blended_mpm_move_with_reference.links.new(
            transform_point.outputs[0], transform_point_001.inputs[0]
        )
        # named_attribute.Attribute -> sample_index.Index
        blended_mpm_move_with_reference.links.new(
            named_attribute.outputs[0], sample_index.inputs[2]
        )
        # group_input_1.Blended MPM Particles -> object_info.Object
        blended_mpm_move_with_reference.links.new(
            group_input_1.outputs[1], object_info.inputs[0]
        )
        # group_input_001.Blended MPM Particles -> blended_mpm_change_of_basis_001.Source Space Obj
        blended_mpm_move_with_reference.links.new(
            group_input_001.outputs[1], blended_mpm_change_of_basis_001.inputs[1]
        )
        # group_input_002.Geometry -> set_position.Geometry
        blended_mpm_move_with_reference.links.new(
            group_input_002.outputs[0], set_position.inputs[0]
        )
        # group_input_002.Visible Transform -> menu_switch.Menu
        blended_mpm_move_with_reference.links.new(
            group_input_002.outputs[2], menu_switch.inputs[0]
        )
        # menu_switch.Output -> set_position.Position
        blended_mpm_move_with_reference.links.new(
            menu_switch.outputs[0], set_position.inputs[2]
        )
        # transform_point.Vector -> menu_switch.Self Object
        blended_mpm_move_with_reference.links.new(
            transform_point.outputs[0], menu_switch.inputs[1]
        )
        # transform_point_001.Vector -> menu_switch.Particle Object
        blended_mpm_move_with_reference.links.new(
            transform_point_001.outputs[0], menu_switch.inputs[2]
        )
        return blended_mpm_move_with_reference

    blended_mpm_move_with_reference = blended_mpm_move_with_reference_node_group()

    # initialize blended_mpm_store_breaking_frame node group
    def blended_mpm_store_breaking_frame_node_group():
        blended_mpm_store_breaking_frame = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Store Breaking Frame"
        )

        blended_mpm_store_breaking_frame.color_tag = "NONE"
        blended_mpm_store_breaking_frame.description = ""
        blended_mpm_store_breaking_frame.default_group_node_width = 140

        blended_mpm_store_breaking_frame.is_modifier = True
        blended_mpm_store_breaking_frame.is_tool = True
        blended_mpm_store_breaking_frame.is_mode_object = False
        blended_mpm_store_breaking_frame.is_mode_edit = False
        blended_mpm_store_breaking_frame.is_mode_sculpt = False
        blended_mpm_store_breaking_frame.is_type_curve = False
        blended_mpm_store_breaking_frame.is_type_mesh = False

        # blended_mpm_store_breaking_frame interface
        # Socket Geometry
        geometry_socket_2 = blended_mpm_store_breaking_frame.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_2.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_3 = blended_mpm_store_breaking_frame.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_3.attribute_domain = "POINT"

        # Socket Num Colliders
        num_colliders_socket = blended_mpm_store_breaking_frame.interface.new_socket(
            name="Num Colliders", in_out="INPUT", socket_type="NodeSocketInt"
        )
        num_colliders_socket.subtype = "NONE"
        num_colliders_socket.attribute_domain = "POINT"

        # Socket Particles
        particles_socket = blended_mpm_store_breaking_frame.interface.new_socket(
            name="Particles", in_out="INPUT", socket_type="NodeSocketObject"
        )
        particles_socket.attribute_domain = "POINT"

        # Socket Dilation Threshold
        dilation_threshold_socket = (
            blended_mpm_store_breaking_frame.interface.new_socket(
                name="Dilation Threshold", in_out="INPUT", socket_type="NodeSocketFloat"
            )
        )
        dilation_threshold_socket.subtype = "NONE"
        dilation_threshold_socket.attribute_domain = "POINT"

        # initialize blended_mpm_store_breaking_frame nodes
        # node Edge Vertices
        edge_vertices = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputMeshEdgeVertices"
        )
        edge_vertices.name = "Edge Vertices"

        # node Sample Index.002
        sample_index_002 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index_002.name = "Sample Index.002"
        sample_index_002.clamp = False
        sample_index_002.data_type = "INT"
        sample_index_002.domain = "POINT"

        # node Group Input.001
        group_input_001_1 = blended_mpm_store_breaking_frame.nodes.new("NodeGroupInput")
        group_input_001_1.name = "Group Input.001"

        # node Sample Index.003
        sample_index_003 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index_003.name = "Sample Index.003"
        sample_index_003.clamp = False
        sample_index_003.data_type = "INT"
        sample_index_003.domain = "POINT"

        # node Named Attribute.004
        named_attribute_004 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_004.name = "Named Attribute.004"
        named_attribute_004.data_type = "INT"
        # Name
        named_attribute_004.inputs[0].default_value = BLENDED_MPM_REFERENCE_INDEX

        # node Math
        math = blended_mpm_store_breaking_frame.nodes.new("ShaderNodeMath")
        math.name = "Math"
        math.operation = "MULTIPLY"
        math.use_clamp = False

        # node Math.001
        math_001 = blended_mpm_store_breaking_frame.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.operation = "LESS_THAN"
        math_001.use_clamp = False
        # Value_001
        math_001.inputs[1].default_value = 0.0

        # node Store Named Attribute
        store_named_attribute = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeStoreNamedAttribute"
        )
        store_named_attribute.name = "Store Named Attribute"
        store_named_attribute.data_type = "INT"
        store_named_attribute.domain = "EDGE"
        # Name
        store_named_attribute.inputs[2].default_value = BLENDED_MPM_BREAKING_FRAME

        # node Scene Time.001
        scene_time_001 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputSceneTime"
        )
        scene_time_001.name = "Scene Time.001"
        scene_time_001.outputs[0].hide = True

        # node Group Output
        group_output_2 = blended_mpm_store_breaking_frame.nodes.new("NodeGroupOutput")
        group_output_2.name = "Group Output"
        group_output_2.is_active_output = True

        # node Repeat Input
        repeat_input = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeRepeatInput"
        )
        repeat_input.name = "Repeat Input"
        # node Repeat Output
        repeat_output = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeRepeatOutput"
        )
        repeat_output.name = "Repeat Output"
        repeat_output.active_index = 0
        repeat_output.inspection_index = 0
        repeat_output.repeat_items.clear()
        # Create item "Value"
        repeat_output.repeat_items.new("BOOLEAN", "Value")

        # node Boolean Math
        boolean_math = blended_mpm_store_breaking_frame.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math.name = "Boolean Math"
        boolean_math.operation = "OR"

        # node Frame
        frame = blended_mpm_store_breaking_frame.nodes.new("NodeFrame")
        frame.label = "Edge broken bc. of some collider?"
        frame.name = "Frame"
        frame.label_size = 20
        frame.shrink = True

        # node Named Attribute
        named_attribute_1 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_1.name = "Named Attribute"
        named_attribute_1.data_type = "FLOAT"
        # Name
        named_attribute_1.inputs[0].default_value = BLENDED_MPM_INITIAL_LENGTH

        # node Edge Vertices.002
        edge_vertices_002 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputMeshEdgeVertices"
        )
        edge_vertices_002.name = "Edge Vertices.002"

        # node Vector Math
        vector_math = blended_mpm_store_breaking_frame.nodes.new("ShaderNodeVectorMath")
        vector_math.name = "Vector Math"
        vector_math.operation = "SUBTRACT"

        # node Vector Math.001
        vector_math_001 = blended_mpm_store_breaking_frame.nodes.new(
            "ShaderNodeVectorMath"
        )
        vector_math_001.name = "Vector Math.001"
        vector_math_001.operation = "LENGTH"

        # node Math.002
        math_002 = blended_mpm_store_breaking_frame.nodes.new("ShaderNodeMath")
        math_002.name = "Math.002"
        math_002.operation = "MULTIPLY"
        math_002.use_clamp = False

        # node Group Input
        group_input_2 = blended_mpm_store_breaking_frame.nodes.new("NodeGroupInput")
        group_input_2.name = "Group Input"

        # node Math.003
        math_003 = blended_mpm_store_breaking_frame.nodes.new("ShaderNodeMath")
        math_003.name = "Math.003"
        math_003.operation = "GREATER_THAN"
        math_003.use_clamp = False

        # node Frame.001
        frame_001 = blended_mpm_store_breaking_frame.nodes.new("NodeFrame")
        frame_001.label = "Edge broken bc. of dilation?"
        frame_001.name = "Frame.001"
        frame_001.label_size = 20
        frame_001.shrink = True

        # node Boolean Math.001
        boolean_math_001 = blended_mpm_store_breaking_frame.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_001.name = "Boolean Math.001"
        boolean_math_001.operation = "OR"

        # node Sample Index.004
        sample_index_004 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index_004.name = "Sample Index.004"
        sample_index_004.clamp = False
        sample_index_004.data_type = "FLOAT"
        sample_index_004.domain = "POINT"

        # node Named Attribute.003
        named_attribute_003 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_003.name = "Named Attribute.003"
        named_attribute_003.data_type = "FLOAT"

        # node Object Info.001
        object_info_001 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_001.name = "Object Info.001"
        object_info_001.hide = True
        object_info_001.transform_space = "ORIGINAL"
        # As Instance
        object_info_001.inputs[1].default_value = False

        # node String
        string = blended_mpm_store_breaking_frame.nodes.new("FunctionNodeInputString")
        string.name = "String"
        string.string = BLENDED_MPM_COLLIDER_INSIDE

        # node Value to String
        value_to_string = blended_mpm_store_breaking_frame.nodes.new(
            "FunctionNodeValueToString"
        )
        value_to_string.name = "Value to String"
        value_to_string.data_type = "INT"

        # node Join Strings
        join_strings = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeStringJoin"
        )
        join_strings.name = "Join Strings"
        # Delimiter
        join_strings.inputs[0].default_value = "_"

        # node Group Input.003
        group_input_003 = blended_mpm_store_breaking_frame.nodes.new("NodeGroupInput")
        group_input_003.name = "Group Input.003"

        # node Sample Index.005
        sample_index_005 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index_005.name = "Sample Index.005"
        sample_index_005.clamp = False
        sample_index_005.data_type = "FLOAT"
        sample_index_005.domain = "POINT"

        # node Reroute
        reroute = blended_mpm_store_breaking_frame.nodes.new("NodeReroute")
        reroute.name = "Reroute"
        reroute.socket_idname = "NodeSocketFloat"
        # node Reroute.001
        reroute_001 = blended_mpm_store_breaking_frame.nodes.new("NodeReroute")
        reroute_001.name = "Reroute.001"
        reroute_001.socket_idname = "NodeSocketFloat"
        # node Reroute.002
        reroute_002 = blended_mpm_store_breaking_frame.nodes.new("NodeReroute")
        reroute_002.name = "Reroute.002"
        reroute_002.socket_idname = "NodeSocketGeometry"
        # node Group Input.002
        group_input_002_1 = blended_mpm_store_breaking_frame.nodes.new("NodeGroupInput")
        group_input_002_1.name = "Group Input.002"

        # node Blended MPM Move With Reference
        blended_mpm_move_with_reference_1 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeGroup"
        )
        blended_mpm_move_with_reference_1.name = "Blended MPM Move With Reference"
        blended_mpm_move_with_reference_1.node_tree = blended_mpm_move_with_reference
        # Socket_3
        blended_mpm_move_with_reference_1.inputs[2].default_value = "Self Object"

        # node Sample Index
        sample_index_1 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index_1.name = "Sample Index"
        sample_index_1.clamp = False
        sample_index_1.data_type = "FLOAT_VECTOR"
        sample_index_1.domain = "POINT"

        # node Position
        position = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputPosition"
        )
        position.name = "Position"

        # node Sample Index.001
        sample_index_001 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeSampleIndex"
        )
        sample_index_001.name = "Sample Index.001"
        sample_index_001.clamp = False
        sample_index_001.data_type = "FLOAT_VECTOR"
        sample_index_001.domain = "POINT"

        # node Group Input.004
        group_input_004 = blended_mpm_store_breaking_frame.nodes.new("NodeGroupInput")
        group_input_004.name = "Group Input.004"

        # node Named Attribute.001
        named_attribute_001_1 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_001_1.name = "Named Attribute.001"
        named_attribute_001_1.data_type = "INT"
        # Name
        named_attribute_001_1.inputs[0].default_value = BLENDED_MPM_BREAKING_FRAME

        # node Math.005
        math_005 = blended_mpm_store_breaking_frame.nodes.new("ShaderNodeMath")
        math_005.name = "Math.005"
        math_005.operation = "LESS_THAN"
        math_005.use_clamp = False

        # node Boolean Math.002
        boolean_math_002 = blended_mpm_store_breaking_frame.nodes.new(
            "FunctionNodeBooleanMath"
        )
        boolean_math_002.name = "Boolean Math.002"
        boolean_math_002.operation = "AND"

        # node Scene Time.002
        scene_time_002 = blended_mpm_store_breaking_frame.nodes.new(
            "GeometryNodeInputSceneTime"
        )
        scene_time_002.name = "Scene Time.002"
        scene_time_002.outputs[0].hide = True

        # Process zone input Repeat Input
        repeat_input.pair_with_output(repeat_output)
        # Item_1
        repeat_input.inputs[1].default_value = False

        # Set parents
        edge_vertices.parent = frame
        sample_index_002.parent = frame
        group_input_001_1.parent = frame
        sample_index_003.parent = frame
        named_attribute_004.parent = frame
        math.parent = frame
        math_001.parent = frame
        repeat_input.parent = frame
        repeat_output.parent = frame
        boolean_math.parent = frame
        named_attribute_1.parent = frame_001
        edge_vertices_002.parent = frame_001
        vector_math.parent = frame_001
        vector_math_001.parent = frame_001
        math_002.parent = frame_001
        group_input_2.parent = frame_001
        math_003.parent = frame_001
        sample_index_004.parent = frame
        named_attribute_003.parent = frame
        object_info_001.parent = frame
        string.parent = frame
        value_to_string.parent = frame
        join_strings.parent = frame
        group_input_003.parent = frame
        sample_index_005.parent = frame
        reroute.parent = frame
        reroute_001.parent = frame
        reroute_002.parent = frame
        group_input_002_1.parent = frame_001
        blended_mpm_move_with_reference_1.parent = frame_001
        sample_index_1.parent = frame_001
        position.parent = frame_001
        sample_index_001.parent = frame_001

        # Set locations
        edge_vertices.location = (90.0, -696.0)
        sample_index_002.location = (350.0, -416.0)
        group_input_001_1.location = (90.0, -376.0)
        sample_index_003.location = (350.0, -636.0)
        named_attribute_004.location = (30.0, -536.0)
        math.location = (1370.0, -276.0)
        math_001.location = (1550.0, -276.0)
        store_named_attribute.location = (3760.0, 180.0)
        scene_time_001.location = (2900.0, 220.0)
        group_output_2.location = (4020.0, 120.0)
        repeat_input.location = (350.0, -176.0)
        repeat_output.location = (1910.0, -176.0)
        boolean_math.location = (1730.0, -176.0)
        frame.location = (670.0, -124.0)
        named_attribute_1.location = (770.0, -216.0)
        edge_vertices_002.location = (210.0, -296.0)
        vector_math.location = (810.0, -56.0)
        vector_math_001.location = (1010.0, -56.0)
        math_002.location = (1010.0, -216.0)
        group_input_2.location = (830.0, -356.0)
        math_003.location = (1210.0, -96.0)
        frame_001.location = (1370.0, 436.0)
        boolean_math_001.location = (2860.0, -40.0)
        sample_index_004.location = (1170.0, -516.0)
        named_attribute_003.location = (910.0, -116.0)
        object_info_001.location = (350.0, -336.0)
        string.location = (470.0, -36.0)
        value_to_string.location = (550.0, -136.0)
        join_strings.location = (730.0, -116.0)
        group_input_003.location = (70.0, -176.0)
        sample_index_005.location = (1170.0, -296.0)
        reroute.location = (1230.0, -216.0)
        reroute_001.location = (890.0, -416.0)
        reroute_002.location = (890.0, -376.0)
        group_input_002_1.location = (30.0, -56.0)
        blended_mpm_move_with_reference_1.location = (210.0, -56.0)
        sample_index_1.location = (590.0, -56.0)
        position.location = (210.0, -216.0)
        sample_index_001.location = (590.0, -276.0)
        group_input_004.location = (3560.0, 240.0)
        named_attribute_001_1.location = (2820.0, 160.0)
        math_005.location = (3120.0, 200.0)
        boolean_math_002.location = (3340.0, 100.0)
        scene_time_002.location = (3560.0, 40.0)

        # Set dimensions
        edge_vertices.width, edge_vertices.height = 140.0, 100.0
        sample_index_002.width, sample_index_002.height = 140.0, 100.0
        group_input_001_1.width, group_input_001_1.height = 140.0, 100.0
        sample_index_003.width, sample_index_003.height = 140.0, 100.0
        named_attribute_004.width, named_attribute_004.height = 200.0, 100.0
        math.width, math.height = 140.0, 100.0
        math_001.width, math_001.height = 140.0, 100.0
        store_named_attribute.width, store_named_attribute.height = 220.0, 100.0
        scene_time_001.width, scene_time_001.height = 140.0, 100.0
        group_output_2.width, group_output_2.height = 140.0, 100.0
        repeat_input.width, repeat_input.height = 140.0, 100.0
        repeat_output.width, repeat_output.height = 140.0, 100.0
        boolean_math.width, boolean_math.height = 140.0, 100.0
        frame.width, frame.height = 2080.0, 859.0
        named_attribute_1.width, named_attribute_1.height = 200.0, 100.0
        edge_vertices_002.width, edge_vertices_002.height = 140.0, 100.0
        vector_math.width, vector_math.height = 140.0, 100.0
        vector_math_001.width, vector_math_001.height = 140.0, 100.0
        math_002.width, math_002.height = 140.0, 100.0
        group_input_2.width, group_input_2.height = 140.0, 100.0
        math_003.width, math_003.height = 140.0, 100.0
        frame_001.width, frame_001.height = 1380.0, 524.0
        boolean_math_001.width, boolean_math_001.height = 140.0, 100.0
        sample_index_004.width, sample_index_004.height = 140.0, 100.0
        named_attribute_003.width, named_attribute_003.height = 240.0, 100.0
        object_info_001.width, object_info_001.height = 140.0, 100.0
        string.width, string.height = 220.0, 100.0
        value_to_string.width, value_to_string.height = 140.0, 100.0
        join_strings.width, join_strings.height = 140.0, 100.0
        group_input_003.width, group_input_003.height = 140.0, 100.0
        sample_index_005.width, sample_index_005.height = 140.0, 100.0
        reroute.width, reroute.height = 10.0, 100.0
        reroute_001.width, reroute_001.height = 10.0, 100.0
        reroute_002.width, reroute_002.height = 10.0, 100.0
        group_input_002_1.width, group_input_002_1.height = 140.0, 100.0
        (
            blended_mpm_move_with_reference_1.width,
            blended_mpm_move_with_reference_1.height,
        ) = 320.0, 100.0
        sample_index_1.width, sample_index_1.height = 140.0, 100.0
        position.width, position.height = 140.0, 100.0
        sample_index_001.width, sample_index_001.height = 140.0, 100.0
        group_input_004.width, group_input_004.height = 140.0, 100.0
        named_attribute_001_1.width, named_attribute_001_1.height = 220.0, 100.0
        math_005.width, math_005.height = 140.0, 100.0
        boolean_math_002.width, boolean_math_002.height = 140.0, 100.0
        scene_time_002.width, scene_time_002.height = 140.0, 100.0

        # initialize blended_mpm_store_breaking_frame links
        # group_input_001_1.Geometry -> sample_index_002.Geometry
        blended_mpm_store_breaking_frame.links.new(
            group_input_001_1.outputs[0], sample_index_002.inputs[0]
        )
        # edge_vertices.Vertex Index 1 -> sample_index_002.Index
        blended_mpm_store_breaking_frame.links.new(
            edge_vertices.outputs[0], sample_index_002.inputs[2]
        )
        # named_attribute_004.Attribute -> sample_index_003.Value
        blended_mpm_store_breaking_frame.links.new(
            named_attribute_004.outputs[0], sample_index_003.inputs[1]
        )
        # named_attribute_004.Attribute -> sample_index_002.Value
        blended_mpm_store_breaking_frame.links.new(
            named_attribute_004.outputs[0], sample_index_002.inputs[1]
        )
        # edge_vertices.Vertex Index 2 -> sample_index_003.Index
        blended_mpm_store_breaking_frame.links.new(
            edge_vertices.outputs[1], sample_index_003.inputs[2]
        )
        # group_input_001_1.Geometry -> sample_index_003.Geometry
        blended_mpm_store_breaking_frame.links.new(
            group_input_001_1.outputs[0], sample_index_003.inputs[0]
        )
        # math.Value -> math_001.Value
        blended_mpm_store_breaking_frame.links.new(math.outputs[0], math_001.inputs[0])
        # boolean_math.Boolean -> repeat_output.Value
        blended_mpm_store_breaking_frame.links.new(
            boolean_math.outputs[0], repeat_output.inputs[0]
        )
        # math_001.Value -> boolean_math.Boolean
        blended_mpm_store_breaking_frame.links.new(
            math_001.outputs[0], boolean_math.inputs[1]
        )
        # repeat_input.Value -> boolean_math.Boolean
        blended_mpm_store_breaking_frame.links.new(
            repeat_input.outputs[1], boolean_math.inputs[0]
        )
        # named_attribute_1.Attribute -> math_002.Value
        blended_mpm_store_breaking_frame.links.new(
            named_attribute_1.outputs[0], math_002.inputs[0]
        )
        # group_input_2.Dilation Threshold -> math_002.Value
        blended_mpm_store_breaking_frame.links.new(
            group_input_2.outputs[3], math_002.inputs[1]
        )
        # vector_math_001.Value -> math_003.Value
        blended_mpm_store_breaking_frame.links.new(
            vector_math_001.outputs[1], math_003.inputs[0]
        )
        # math_002.Value -> math_003.Value
        blended_mpm_store_breaking_frame.links.new(
            math_002.outputs[0], math_003.inputs[1]
        )
        # repeat_output.Value -> boolean_math_001.Boolean
        blended_mpm_store_breaking_frame.links.new(
            repeat_output.outputs[0], boolean_math_001.inputs[1]
        )
        # store_named_attribute.Geometry -> group_output_2.Geometry
        blended_mpm_store_breaking_frame.links.new(
            store_named_attribute.outputs[0], group_output_2.inputs[0]
        )
        # sample_index_003.Value -> sample_index_004.Index
        blended_mpm_store_breaking_frame.links.new(
            sample_index_003.outputs[0], sample_index_004.inputs[2]
        )
        # value_to_string.String -> join_strings.Strings
        blended_mpm_store_breaking_frame.links.new(
            value_to_string.outputs[0], join_strings.inputs[1]
        )
        # join_strings.String -> named_attribute_003.Name
        blended_mpm_store_breaking_frame.links.new(
            join_strings.outputs[0], named_attribute_003.inputs[0]
        )
        # repeat_input.Iteration -> value_to_string.Value
        blended_mpm_store_breaking_frame.links.new(
            repeat_input.outputs[0], value_to_string.inputs[0]
        )
        # sample_index_004.Value -> math.Value
        blended_mpm_store_breaking_frame.links.new(
            sample_index_004.outputs[0], math.inputs[1]
        )
        # group_input_003.Num Colliders -> repeat_input.Iterations
        blended_mpm_store_breaking_frame.links.new(
            group_input_003.outputs[1], repeat_input.inputs[0]
        )
        # group_input_003.Particles -> object_info_001.Object
        blended_mpm_store_breaking_frame.links.new(
            group_input_003.outputs[2], object_info_001.inputs[0]
        )
        # sample_index_002.Value -> sample_index_005.Index
        blended_mpm_store_breaking_frame.links.new(
            sample_index_002.outputs[0], sample_index_005.inputs[2]
        )
        # sample_index_005.Value -> math.Value
        blended_mpm_store_breaking_frame.links.new(
            sample_index_005.outputs[0], math.inputs[0]
        )
        # named_attribute_003.Attribute -> reroute.Input
        blended_mpm_store_breaking_frame.links.new(
            named_attribute_003.outputs[0], reroute.inputs[0]
        )
        # reroute.Output -> reroute_001.Input
        blended_mpm_store_breaking_frame.links.new(
            reroute.outputs[0], reroute_001.inputs[0]
        )
        # reroute_001.Output -> sample_index_005.Value
        blended_mpm_store_breaking_frame.links.new(
            reroute_001.outputs[0], sample_index_005.inputs[1]
        )
        # reroute_001.Output -> sample_index_004.Value
        blended_mpm_store_breaking_frame.links.new(
            reroute_001.outputs[0], sample_index_004.inputs[1]
        )
        # object_info_001.Geometry -> reroute_002.Input
        blended_mpm_store_breaking_frame.links.new(
            object_info_001.outputs[4], reroute_002.inputs[0]
        )
        # reroute_002.Output -> sample_index_005.Geometry
        blended_mpm_store_breaking_frame.links.new(
            reroute_002.outputs[0], sample_index_005.inputs[0]
        )
        # reroute_002.Output -> sample_index_004.Geometry
        blended_mpm_store_breaking_frame.links.new(
            reroute_002.outputs[0], sample_index_004.inputs[0]
        )
        # boolean_math_002.Boolean -> store_named_attribute.Selection
        blended_mpm_store_breaking_frame.links.new(
            boolean_math_002.outputs[0], store_named_attribute.inputs[1]
        )
        # group_input_002_1.Particles -> blended_mpm_move_with_reference_1.Blended MPM Particles
        blended_mpm_store_breaking_frame.links.new(
            group_input_002_1.outputs[2], blended_mpm_move_with_reference_1.inputs[1]
        )
        # group_input_002_1.Geometry -> blended_mpm_move_with_reference_1.Geometry
        blended_mpm_store_breaking_frame.links.new(
            group_input_002_1.outputs[0], blended_mpm_move_with_reference_1.inputs[0]
        )
        # blended_mpm_move_with_reference_1.Geometry -> sample_index_1.Geometry
        blended_mpm_store_breaking_frame.links.new(
            blended_mpm_move_with_reference_1.outputs[0], sample_index_1.inputs[0]
        )
        # position.Position -> sample_index_1.Value
        blended_mpm_store_breaking_frame.links.new(
            position.outputs[0], sample_index_1.inputs[1]
        )
        # edge_vertices_002.Vertex Index 1 -> sample_index_1.Index
        blended_mpm_store_breaking_frame.links.new(
            edge_vertices_002.outputs[0], sample_index_1.inputs[2]
        )
        # sample_index_1.Value -> vector_math.Vector
        blended_mpm_store_breaking_frame.links.new(
            sample_index_1.outputs[0], vector_math.inputs[0]
        )
        # edge_vertices_002.Vertex Index 2 -> sample_index_001.Index
        blended_mpm_store_breaking_frame.links.new(
            edge_vertices_002.outputs[1], sample_index_001.inputs[2]
        )
        # blended_mpm_move_with_reference_1.Geometry -> sample_index_001.Geometry
        blended_mpm_store_breaking_frame.links.new(
            blended_mpm_move_with_reference_1.outputs[0], sample_index_001.inputs[0]
        )
        # position.Position -> sample_index_001.Value
        blended_mpm_store_breaking_frame.links.new(
            position.outputs[0], sample_index_001.inputs[1]
        )
        # sample_index_001.Value -> vector_math.Vector
        blended_mpm_store_breaking_frame.links.new(
            sample_index_001.outputs[0], vector_math.inputs[1]
        )
        # group_input_004.Geometry -> store_named_attribute.Geometry
        blended_mpm_store_breaking_frame.links.new(
            group_input_004.outputs[0], store_named_attribute.inputs[0]
        )
        # boolean_math_001.Boolean -> boolean_math_002.Boolean
        blended_mpm_store_breaking_frame.links.new(
            boolean_math_001.outputs[0], boolean_math_002.inputs[1]
        )
        # math_005.Value -> boolean_math_002.Boolean
        blended_mpm_store_breaking_frame.links.new(
            math_005.outputs[0], boolean_math_002.inputs[0]
        )
        # scene_time_002.Frame -> store_named_attribute.Value
        blended_mpm_store_breaking_frame.links.new(
            scene_time_002.outputs[1], store_named_attribute.inputs[3]
        )
        # scene_time_001.Frame -> math_005.Value
        blended_mpm_store_breaking_frame.links.new(
            scene_time_001.outputs[1], math_005.inputs[0]
        )
        # named_attribute_001_1.Attribute -> math_005.Value
        blended_mpm_store_breaking_frame.links.new(
            named_attribute_001_1.outputs[0], math_005.inputs[1]
        )
        # math_003.Value -> boolean_math_001.Boolean
        blended_mpm_store_breaking_frame.links.new(
            math_003.outputs[0], boolean_math_001.inputs[0]
        )
        # vector_math.Vector -> vector_math_001.Vector
        blended_mpm_store_breaking_frame.links.new(
            vector_math.outputs[0], vector_math_001.inputs[0]
        )
        # string.String -> join_strings.Strings
        blended_mpm_store_breaking_frame.links.new(
            string.outputs[0], join_strings.inputs[1]
        )
        return blended_mpm_store_breaking_frame

    return blended_mpm_store_breaking_frame_node_group()
