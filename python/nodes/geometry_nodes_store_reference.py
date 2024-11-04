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
    BLENDED_MPM_INITIAL_LENGTH,
    BLENDED_MPM_REFERENCE_INDEX,
    BLENDED_MPM_REFERENCE_OFFSET,
    BLENDED_MPM_TRANSFORM,
)


def create_geometry_nodes_store_reference():
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

    # initialize blended_mpm_store_reference node group
    def blended_mpm_store_reference_node_group():
        blended_mpm_store_reference = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Blended MPM Store Reference"
        )

        blended_mpm_store_reference.color_tag = "NONE"
        blended_mpm_store_reference.description = ""
        blended_mpm_store_reference.default_group_node_width = 140

        blended_mpm_store_reference.is_modifier = True

        # blended_mpm_store_reference interface
        # Socket Geometry
        geometry_socket = blended_mpm_store_reference.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = blended_mpm_store_reference.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Blended MPM Particles
        blended_mpm_particles_socket = blended_mpm_store_reference.interface.new_socket(
            name="Blended MPM Particles", in_out="INPUT", socket_type="NodeSocketObject"
        )
        blended_mpm_particles_socket.attribute_domain = "POINT"

        # initialize blended_mpm_store_reference nodes
        # node Group Input
        group_input_1 = blended_mpm_store_reference.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Group Output
        group_output_1 = blended_mpm_store_reference.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Store Named Attribute
        store_named_attribute = blended_mpm_store_reference.nodes.new(
            "GeometryNodeStoreNamedAttribute"
        )
        store_named_attribute.name = "Store Named Attribute"
        store_named_attribute.data_type = "INT"
        store_named_attribute.domain = "POINT"
        # Selection
        store_named_attribute.inputs[1].default_value = True
        # Name
        store_named_attribute.inputs[2].default_value = BLENDED_MPM_REFERENCE_INDEX

        # node Sample Index
        sample_index = blended_mpm_store_reference.nodes.new("GeometryNodeSampleIndex")
        sample_index.name = "Sample Index"
        sample_index.clamp = False
        sample_index.data_type = "FLOAT4X4"
        sample_index.domain = "POINT"

        # node Named Attribute
        named_attribute = blended_mpm_store_reference.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "FLOAT4X4"
        # Name
        named_attribute.inputs[0].default_value = BLENDED_MPM_TRANSFORM

        # node Store Named Attribute.001
        store_named_attribute_001 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeStoreNamedAttribute"
        )
        store_named_attribute_001.name = "Store Named Attribute.001"
        store_named_attribute_001.data_type = "FLOAT_VECTOR"
        store_named_attribute_001.domain = "POINT"
        # Selection
        store_named_attribute_001.inputs[1].default_value = True
        # Name
        store_named_attribute_001.inputs[2].default_value = BLENDED_MPM_REFERENCE_OFFSET

        # node Invert Matrix
        invert_matrix = blended_mpm_store_reference.nodes.new(
            "FunctionNodeInvertMatrix"
        )
        invert_matrix.name = "Invert Matrix"

        # node Transform Point
        transform_point = blended_mpm_store_reference.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point.name = "Transform Point"

        # node Store Named Attribute.002
        store_named_attribute_002 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeStoreNamedAttribute"
        )
        store_named_attribute_002.name = "Store Named Attribute.002"
        store_named_attribute_002.data_type = "FLOAT"
        store_named_attribute_002.domain = "EDGE"
        # Selection
        store_named_attribute_002.inputs[1].default_value = True
        # Name
        store_named_attribute_002.inputs[2].default_value = BLENDED_MPM_INITIAL_LENGTH

        # node Edge Vertices
        edge_vertices = blended_mpm_store_reference.nodes.new(
            "GeometryNodeInputMeshEdgeVertices"
        )
        edge_vertices.name = "Edge Vertices"

        # node Vector Math
        vector_math = blended_mpm_store_reference.nodes.new("ShaderNodeVectorMath")
        vector_math.name = "Vector Math"
        vector_math.operation = "SUBTRACT"

        # node Vector Math.001
        vector_math_001 = blended_mpm_store_reference.nodes.new("ShaderNodeVectorMath")
        vector_math_001.name = "Vector Math.001"
        vector_math_001.operation = "LENGTH"

        # node Transform Point.001
        transform_point_001 = blended_mpm_store_reference.nodes.new(
            "FunctionNodeTransformPoint"
        )
        transform_point_001.name = "Transform Point.001"

        # node Position.002
        position_002 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeInputPosition"
        )
        position_002.name = "Position.002"

        # node Frame
        frame = blended_mpm_store_reference.nodes.new("NodeFrame")
        frame.label = "Transform to Particle Object Space"
        frame.name = "Frame"
        frame.label_size = 20
        frame.shrink = True

        # node Group Input.001
        group_input_001 = blended_mpm_store_reference.nodes.new("NodeGroupInput")
        group_input_001.name = "Group Input.001"

        # node Group Input.002
        group_input_002 = blended_mpm_store_reference.nodes.new("NodeGroupInput")
        group_input_002.name = "Group Input.002"

        # node Object Info.003
        object_info_003 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_003.name = "Object Info.003"
        object_info_003.transform_space = "ORIGINAL"
        # As Instance
        object_info_003.inputs[1].default_value = False

        # node Sample Nearest.001
        sample_nearest_001 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeSampleNearest"
        )
        sample_nearest_001.name = "Sample Nearest.001"
        sample_nearest_001.domain = "POINT"

        # node Group Input.003
        group_input_003 = blended_mpm_store_reference.nodes.new("NodeGroupInput")
        group_input_003.name = "Group Input.003"

        # node Object Info.004
        object_info_004 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeObjectInfo"
        )
        object_info_004.name = "Object Info.004"
        object_info_004.transform_space = "ORIGINAL"
        # As Instance
        object_info_004.inputs[1].default_value = False

        # node Frame.001
        frame_001 = blended_mpm_store_reference.nodes.new("NodeFrame")
        frame_001.label = "Transform to Reference Particle Space"
        frame_001.name = "Frame.001"
        frame_001.label_size = 20
        frame_001.shrink = True

        # node Reroute
        reroute = blended_mpm_store_reference.nodes.new("NodeReroute")
        reroute.name = "Reroute"
        reroute.socket_idname = "NodeSocketInt"
        # node Reroute.001
        reroute_001 = blended_mpm_store_reference.nodes.new("NodeReroute")
        reroute_001.name = "Reroute.001"
        reroute_001.socket_idname = "NodeSocketInt"
        # node Group
        group = blended_mpm_store_reference.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = blended_mpm_change_of_basis

        # node Self Object
        self_object = blended_mpm_store_reference.nodes.new("GeometryNodeSelfObject")
        self_object.name = "Self Object"

        # node Group Input.004
        group_input_004 = blended_mpm_store_reference.nodes.new("NodeGroupInput")
        group_input_004.name = "Group Input.004"

        # node Group.002
        group_002 = blended_mpm_store_reference.nodes.new("GeometryNodeGroup")
        group_002.name = "Group.002"
        group_002.node_tree = blended_mpm_change_of_basis

        # node Self Object.001
        self_object_001 = blended_mpm_store_reference.nodes.new(
            "GeometryNodeSelfObject"
        )
        self_object_001.name = "Self Object.001"

        # node Transform Direction
        transform_direction = blended_mpm_store_reference.nodes.new(
            "FunctionNodeTransformDirection"
        )
        transform_direction.name = "Transform Direction"

        # node Frame.002
        frame_002 = blended_mpm_store_reference.nodes.new("NodeFrame")
        frame_002.label = "Measure Edge Lengh in Particle Object Space"
        frame_002.name = "Frame.002"
        frame_002.label_size = 20
        frame_002.shrink = True

        # node Reroute.002
        reroute_002 = blended_mpm_store_reference.nodes.new("NodeReroute")
        reroute_002.name = "Reroute.002"
        reroute_002.socket_idname = "NodeSocketVectorXYZ"
        # node Reroute.003
        reroute_003 = blended_mpm_store_reference.nodes.new("NodeReroute")
        reroute_003.name = "Reroute.003"
        reroute_003.socket_idname = "NodeSocketVectorXYZ"

        # Set parents
        sample_index.parent = frame_001
        named_attribute.parent = frame_001
        invert_matrix.parent = frame_001
        transform_point.parent = frame_001
        edge_vertices.parent = frame_002
        vector_math.parent = frame_002
        vector_math_001.parent = frame_002
        transform_point_001.parent = frame
        position_002.parent = frame
        group_input_001.parent = frame
        group_input_003.parent = frame_001
        object_info_004.parent = frame_001
        reroute_001.parent = frame_001
        group.parent = frame
        self_object.parent = frame
        group_input_004.parent = frame_002
        group_002.parent = frame_002
        self_object_001.parent = frame_002
        transform_direction.parent = frame_002

        # Set locations
        group_input_1.location = (-1080.0, -300.0)
        group_output_1.location = (1640.0, -300.0)
        store_named_attribute.location = (-860.0, -300.0)
        sample_index.location = (510.0, -96.0)
        named_attribute.location = (30.0, -236.0)
        store_named_attribute_001.location = (300.0, -300.0)
        invert_matrix.location = (690.0, -96.0)
        transform_point.location = (870.0, -36.0)
        store_named_attribute_002.location = (1340.0, -300.0)
        edge_vertices.location = (210.0, -36.0)
        vector_math.location = (390.0, -36.0)
        vector_math_001.location = (770.0, -36.0)
        transform_point_001.location = (750.0, -56.0)
        position_002.location = (570.0, -36.0)
        frame.location = (-2050.0, -624.0)
        group_input_001.location = (30.0, -56.0)
        group_input_002.location = (-1460.0, -380.0)
        object_info_003.location = (-1280.0, -280.0)
        sample_nearest_001.location = (-1080.0, -440.0)
        group_input_003.location = (110.0, -96.0)
        object_info_004.location = (310.0, -36.0)
        frame_001.location = (-810.0, -524.0)
        reroute.location = (-820.0, -920.0)
        reroute_001.location = (290.0, -396.0)
        group.location = (250.0, -76.0)
        self_object.location = (30.0, -176.0)
        group_input_004.location = (30.0, -176.0)
        group_002.location = (250.0, -196.0)
        self_object_001.location = (30.0, -296.0)
        transform_direction.location = (590.0, -96.0)
        frame_002.location = (330.0, -524.0)
        reroute_002.location = (-800.0, -500.0)
        reroute_003.location = (-60.0, -500.0)

        # Set dimensions
        group_input_1.width, group_input_1.height = 140.0, 100.0
        group_output_1.width, group_output_1.height = 140.0, 100.0
        store_named_attribute.width, store_named_attribute.height = 220.0, 100.0
        sample_index.width, sample_index.height = 140.0, 100.0
        named_attribute.width, named_attribute.height = 260.0, 100.0
        store_named_attribute_001.width, store_named_attribute_001.height = 240.0, 100.0
        invert_matrix.width, invert_matrix.height = 140.0, 100.0
        transform_point.width, transform_point.height = 140.0, 100.0
        store_named_attribute_002.width, store_named_attribute_002.height = 220.0, 100.0
        edge_vertices.width, edge_vertices.height = 140.0, 100.0
        vector_math.width, vector_math.height = 140.0, 100.0
        vector_math_001.width, vector_math_001.height = 140.0, 100.0
        transform_point_001.width, transform_point_001.height = 140.0, 100.0
        position_002.width, position_002.height = 140.0, 100.0
        frame.width, frame.height = 920.0, 256.0
        group_input_001.width, group_input_001.height = 140.0, 100.0
        group_input_002.width, group_input_002.height = 140.0, 100.0
        object_info_003.width, object_info_003.height = 140.0, 100.0
        sample_nearest_001.width, sample_nearest_001.height = 140.0, 100.0
        group_input_003.width, group_input_003.height = 140.0, 100.0
        object_info_004.width, object_info_004.height = 140.0, 100.0
        frame_001.width, frame_001.height = 1040.0, 431.0
        reroute.width, reroute.height = 10.0, 100.0
        reroute_001.width, reroute_001.height = 10.0, 100.0
        group.width, group.height = 280.0, 100.0
        self_object.width, self_object.height = 140.0, 100.0
        group_input_004.width, group_input_004.height = 140.0, 100.0
        group_002.width, group_002.height = 280.0, 100.0
        self_object_001.width, self_object_001.height = 140.0, 100.0
        transform_direction.width, transform_direction.height = 140.0, 100.0
        frame_002.width, frame_002.height = 940.0, 376.0
        reroute_002.width, reroute_002.height = 10.0, 100.0
        reroute_003.width, reroute_003.height = 10.0, 100.0

        # initialize blended_mpm_store_reference links
        # group_input_1.Geometry -> store_named_attribute.Geometry
        blended_mpm_store_reference.links.new(
            group_input_1.outputs[0], store_named_attribute.inputs[0]
        )
        # named_attribute.Attribute -> sample_index.Value
        blended_mpm_store_reference.links.new(
            named_attribute.outputs[0], sample_index.inputs[1]
        )
        # sample_index.Value -> invert_matrix.Matrix
        blended_mpm_store_reference.links.new(
            sample_index.outputs[0], invert_matrix.inputs[0]
        )
        # invert_matrix.Matrix -> transform_point.Transform
        blended_mpm_store_reference.links.new(
            invert_matrix.outputs[0], transform_point.inputs[1]
        )
        # transform_point.Vector -> store_named_attribute_001.Value
        blended_mpm_store_reference.links.new(
            transform_point.outputs[0], store_named_attribute_001.inputs[3]
        )
        # store_named_attribute.Geometry -> store_named_attribute_001.Geometry
        blended_mpm_store_reference.links.new(
            store_named_attribute.outputs[0], store_named_attribute_001.inputs[0]
        )
        # edge_vertices.Position 2 -> vector_math.Vector
        blended_mpm_store_reference.links.new(
            edge_vertices.outputs[3], vector_math.inputs[1]
        )
        # vector_math_001.Value -> store_named_attribute_002.Value
        blended_mpm_store_reference.links.new(
            vector_math_001.outputs[1], store_named_attribute_002.inputs[3]
        )
        # store_named_attribute_001.Geometry -> store_named_attribute_002.Geometry
        blended_mpm_store_reference.links.new(
            store_named_attribute_001.outputs[0], store_named_attribute_002.inputs[0]
        )
        # store_named_attribute_002.Geometry -> group_output_1.Geometry
        blended_mpm_store_reference.links.new(
            store_named_attribute_002.outputs[0], group_output_1.inputs[0]
        )
        # position_002.Position -> transform_point_001.Vector
        blended_mpm_store_reference.links.new(
            position_002.outputs[0], transform_point_001.inputs[0]
        )
        # group_input_002.Blended MPM Particles -> object_info_003.Object
        blended_mpm_store_reference.links.new(
            group_input_002.outputs[1], object_info_003.inputs[0]
        )
        # object_info_003.Geometry -> sample_nearest_001.Geometry
        blended_mpm_store_reference.links.new(
            object_info_003.outputs[4], sample_nearest_001.inputs[0]
        )
        # transform_point_001.Vector -> sample_nearest_001.Sample Position
        blended_mpm_store_reference.links.new(
            transform_point_001.outputs[0], sample_nearest_001.inputs[1]
        )
        # sample_nearest_001.Index -> store_named_attribute.Value
        blended_mpm_store_reference.links.new(
            sample_nearest_001.outputs[0], store_named_attribute.inputs[3]
        )
        # reroute_001.Output -> sample_index.Index
        blended_mpm_store_reference.links.new(
            reroute_001.outputs[0], sample_index.inputs[2]
        )
        # reroute_003.Output -> transform_point.Vector
        blended_mpm_store_reference.links.new(
            reroute_003.outputs[0], transform_point.inputs[0]
        )
        # group_input_003.Blended MPM Particles -> object_info_004.Object
        blended_mpm_store_reference.links.new(
            group_input_003.outputs[1], object_info_004.inputs[0]
        )
        # object_info_004.Geometry -> sample_index.Geometry
        blended_mpm_store_reference.links.new(
            object_info_004.outputs[4], sample_index.inputs[0]
        )
        # sample_nearest_001.Index -> reroute.Input
        blended_mpm_store_reference.links.new(
            sample_nearest_001.outputs[0], reroute.inputs[0]
        )
        # reroute.Output -> reroute_001.Input
        blended_mpm_store_reference.links.new(reroute.outputs[0], reroute_001.inputs[0])
        # group.Transform -> transform_point_001.Transform
        blended_mpm_store_reference.links.new(
            group.outputs[0], transform_point_001.inputs[1]
        )
        # group_input_001.Blended MPM Particles -> group.Target Space Obj
        blended_mpm_store_reference.links.new(
            group_input_001.outputs[1], group.inputs[0]
        )
        # self_object.Self Object -> group.Source Space Obj
        blended_mpm_store_reference.links.new(self_object.outputs[0], group.inputs[1])
        # group_input_004.Blended MPM Particles -> group_002.Target Space Obj
        blended_mpm_store_reference.links.new(
            group_input_004.outputs[1], group_002.inputs[0]
        )
        # self_object_001.Self Object -> group_002.Source Space Obj
        blended_mpm_store_reference.links.new(
            self_object_001.outputs[0], group_002.inputs[1]
        )
        # edge_vertices.Position 1 -> vector_math.Vector
        blended_mpm_store_reference.links.new(
            edge_vertices.outputs[2], vector_math.inputs[0]
        )
        # group_002.Transform -> transform_direction.Transform
        blended_mpm_store_reference.links.new(
            group_002.outputs[0], transform_direction.inputs[1]
        )
        # vector_math.Vector -> transform_direction.Direction
        blended_mpm_store_reference.links.new(
            vector_math.outputs[0], transform_direction.inputs[0]
        )
        # transform_direction.Direction -> vector_math_001.Vector
        blended_mpm_store_reference.links.new(
            transform_direction.outputs[0], vector_math_001.inputs[0]
        )
        # transform_point_001.Vector -> reroute_002.Input
        blended_mpm_store_reference.links.new(
            transform_point_001.outputs[0], reroute_002.inputs[0]
        )
        # reroute_002.Output -> reroute_003.Input
        blended_mpm_store_reference.links.new(
            reroute_002.outputs[0], reroute_003.inputs[0]
        )
        return blended_mpm_store_reference

    return blended_mpm_store_reference_node_group()
