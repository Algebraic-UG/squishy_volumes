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

from .material_colored_instances import create_material_colored_instances
from ..magic_consts import (
    SQUISHY_VOLUMES_NORMAL,
    SQUISHY_VOLUMES_DISTANCE,
    SQUISHY_VOLUMES_INSTANCE_COLOR,
)


def create_geometry_nodes_grid_distance():
    material_colored_instances = create_material_colored_instances()

    # initialize squishy_volumes_color_instance node group
    def squishy_volumes_color_instance_node_group():
        squishy_volumes_color_instance = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Color Instance"
        )

        squishy_volumes_color_instance.color_tag = "NONE"
        squishy_volumes_color_instance.description = ""
        squishy_volumes_color_instance.default_group_node_width = 140

        # squishy_volumes_color_instance interface
        # Socket Geometry
        geometry_socket_2 = squishy_volumes_color_instance.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_2.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_3 = squishy_volumes_color_instance.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_3.attribute_domain = "POINT"

        # Socket Instance Color
        instance_color_socket_2 = squishy_volumes_color_instance.interface.new_socket(
            name="Instance Color", in_out="INPUT", socket_type="NodeSocketColor"
        )
        instance_color_socket_2.default_value = (0.0, 0.0, 0.0, 1.0)
        instance_color_socket_2.attribute_domain = "POINT"

        # initialize squishy_volumes_color_instance nodes
        # node Group Input
        group_input_3 = squishy_volumes_color_instance.nodes.new("NodeGroupInput")
        group_input_3.name = "Group Input"

        # node Store Named Attribute
        store_named_attribute = squishy_volumes_color_instance.nodes.new(
            "GeometryNodeStoreNamedAttribute"
        )
        store_named_attribute.name = "Store Named Attribute"
        store_named_attribute.data_type = "FLOAT_COLOR"
        store_named_attribute.domain = "INSTANCE"
        # Selection
        store_named_attribute.inputs[1].default_value = True
        # Name
        store_named_attribute.inputs[2].default_value = SQUISHY_VOLUMES_INSTANCE_COLOR

        # node Set Material
        set_material = squishy_volumes_color_instance.nodes.new(
            "GeometryNodeSetMaterial"
        )
        set_material.name = "Set Material"
        # Selection
        set_material.inputs[1].default_value = True
        set_material.inputs[2].default_value = material_colored_instances

        # node Group Output
        group_output_3 = squishy_volumes_color_instance.nodes.new("NodeGroupOutput")
        group_output_3.name = "Group Output"
        group_output_3.is_active_output = True

        # Set locations
        group_input_3.location = (0.0, 0.0)
        store_named_attribute.location = (300.0, 0.0)
        set_material.location = (600.0, 0.0)
        group_output_3.location = (900.0, 0.0)

        # Set dimensions
        group_input_3.width, group_input_3.height = 140.0, 100.0
        store_named_attribute.width, store_named_attribute.height = 250.0, 100.0
        set_material.width, set_material.height = 250.0, 100.0
        group_output_3.width, group_output_3.height = 140.0, 100.0

        # initialize squishy_volumes_color_instance links
        # group_input_3.Geometry -> store_named_attribute.Geometry
        squishy_volumes_color_instance.links.new(
            group_input_3.outputs[0], store_named_attribute.inputs[0]
        )
        # group_input_3.Instance Color -> store_named_attribute.Value
        squishy_volumes_color_instance.links.new(
            group_input_3.outputs[1], store_named_attribute.inputs[3]
        )
        # store_named_attribute.Geometry -> set_material.Geometry
        squishy_volumes_color_instance.links.new(
            store_named_attribute.outputs[0], set_material.inputs[0]
        )
        # set_material.Geometry -> group_output_3.Geometry
        squishy_volumes_color_instance.links.new(
            set_material.outputs[0], group_output_3.inputs[0]
        )
        return squishy_volumes_color_instance

    squishy_volumes_color_instance = squishy_volumes_color_instance_node_group()

    # initialize squishy_volumes_vector node group
    def squishy_volumes_vector_node_group():
        squishy_volumes_vector = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Vector"
        )

        squishy_volumes_vector.color_tag = "NONE"
        squishy_volumes_vector.description = ""
        squishy_volumes_vector.default_group_node_width = 140

        # squishy_volumes_vector interface
        # Socket Instances
        instances_socket = squishy_volumes_vector.interface.new_socket(
            name="Instances", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        instances_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_2 = squishy_volumes_vector.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_2.attribute_domain = "POINT"

        # Socket Vector
        vector_socket = squishy_volumes_vector.interface.new_socket(
            name="Vector", in_out="INPUT", socket_type="NodeSocketVector"
        )
        vector_socket.default_value = (0.0, 0.0, 1.0)
        vector_socket.min_value = -3.4028234663852886e38
        vector_socket.max_value = 3.4028234663852886e38
        vector_socket.subtype = "XYZ"
        vector_socket.attribute_domain = "POINT"

        # Socket Scale
        scale_socket = squishy_volumes_vector.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket.default_value = 1.0
        scale_socket.min_value = -10000.0
        scale_socket.max_value = 10000.0
        scale_socket.subtype = "NONE"
        scale_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_vector nodes
        # node Group Output
        group_output_1 = squishy_volumes_vector.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Group Input
        group_input_1 = squishy_volumes_vector.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Mesh Line
        mesh_line = squishy_volumes_vector.nodes.new("GeometryNodeMeshLine")
        mesh_line.name = "Mesh Line"
        mesh_line.count_mode = "TOTAL"
        mesh_line.mode = "OFFSET"
        # Count
        mesh_line.inputs[0].default_value = 2
        # Start Location
        mesh_line.inputs[2].default_value = (0.0, 0.0, 0.0)
        # Offset
        mesh_line.inputs[3].default_value = (1.0, 0.0, 0.0)

        # node Align Rotation to Vector
        align_rotation_to_vector = squishy_volumes_vector.nodes.new(
            "FunctionNodeAlignRotationToVector"
        )
        align_rotation_to_vector.name = "Align Rotation to Vector"
        align_rotation_to_vector.axis = "X"
        align_rotation_to_vector.pivot_axis = "AUTO"
        # Rotation
        align_rotation_to_vector.inputs[0].default_value = (0.0, 0.0, 0.0)
        # Factor
        align_rotation_to_vector.inputs[1].default_value = 1.0

        # node Vector Math
        vector_math = squishy_volumes_vector.nodes.new("ShaderNodeVectorMath")
        vector_math.name = "Vector Math"
        vector_math.operation = "LENGTH"

        # node Instance on Points
        instance_on_points = squishy_volumes_vector.nodes.new(
            "GeometryNodeInstanceOnPoints"
        )
        instance_on_points.name = "Instance on Points"
        # Selection
        instance_on_points.inputs[1].default_value = True
        # Pick Instance
        instance_on_points.inputs[3].default_value = False
        # Instance Index
        instance_on_points.inputs[4].default_value = 0
        # Rotation
        instance_on_points.inputs[5].default_value = (0.0, 0.0, 0.0)

        # node Math
        math = squishy_volumes_vector.nodes.new("ShaderNodeMath")
        math.name = "Math"
        math.operation = "MULTIPLY"
        math.use_clamp = False

        # node Rotate Instances
        rotate_instances = squishy_volumes_vector.nodes.new(
            "GeometryNodeRotateInstances"
        )
        rotate_instances.name = "Rotate Instances"
        # Selection
        rotate_instances.inputs[1].default_value = True
        # Pivot Point
        rotate_instances.inputs[3].default_value = (0.0, 0.0, 0.0)
        # Local Space
        rotate_instances.inputs[4].default_value = True

        # Set locations
        group_output_1.location = (930.9354248046875, 183.57730102539062)
        group_input_1.location = (-399.0645751953125, 205.57730102539062)
        mesh_line.location = (170.9354248046875, 281.0773010253906)
        align_rotation_to_vector.location = (550.9354248046875, 231.57730102539062)
        vector_math.location = (-209.0645751953125, 198.07730102539062)
        instance_on_points.location = (360.9354248046875, 282.0773010253906)
        math.location = (-19.0645751953125, 221.57730102539062)
        rotate_instances.location = (740.9354248046875, 260.0773010253906)

        # Set dimensions
        group_output_1.width, group_output_1.height = 140.0, 100.0
        group_input_1.width, group_input_1.height = 140.0, 100.0
        mesh_line.width, mesh_line.height = 140.0, 100.0
        align_rotation_to_vector.width, align_rotation_to_vector.height = 140.0, 100.0
        vector_math.width, vector_math.height = 140.0, 100.0
        instance_on_points.width, instance_on_points.height = 140.0, 100.0
        math.width, math.height = 140.0, 100.0
        rotate_instances.width, rotate_instances.height = 140.0, 100.0

        # initialize squishy_volumes_vector links
        # group_input_1.Vector -> align_rotation_to_vector.Vector
        squishy_volumes_vector.links.new(
            group_input_1.outputs[1], align_rotation_to_vector.inputs[2]
        )
        # group_input_1.Vector -> vector_math.Vector
        squishy_volumes_vector.links.new(
            group_input_1.outputs[1], vector_math.inputs[0]
        )
        # mesh_line.Mesh -> instance_on_points.Instance
        squishy_volumes_vector.links.new(
            mesh_line.outputs[0], instance_on_points.inputs[2]
        )
        # vector_math.Value -> math.Value
        squishy_volumes_vector.links.new(vector_math.outputs[1], math.inputs[0])
        # group_input_1.Scale -> math.Value
        squishy_volumes_vector.links.new(group_input_1.outputs[2], math.inputs[1])
        # group_input_1.Geometry -> instance_on_points.Points
        squishy_volumes_vector.links.new(
            group_input_1.outputs[0], instance_on_points.inputs[0]
        )
        # instance_on_points.Instances -> rotate_instances.Instances
        squishy_volumes_vector.links.new(
            instance_on_points.outputs[0], rotate_instances.inputs[0]
        )
        # rotate_instances.Instances -> group_output_1.Instances
        squishy_volumes_vector.links.new(
            rotate_instances.outputs[0], group_output_1.inputs[0]
        )
        # align_rotation_to_vector.Rotation -> rotate_instances.Rotation
        squishy_volumes_vector.links.new(
            align_rotation_to_vector.outputs[0], rotate_instances.inputs[2]
        )
        # math.Value -> instance_on_points.Scale
        squishy_volumes_vector.links.new(math.outputs[0], instance_on_points.inputs[6])
        return squishy_volumes_vector

    squishy_volumes_vector = squishy_volumes_vector_node_group()

    # initialize squishy_volumes_crystal_grid node group
    def squishy_volumes_crystal_grid_node_group():
        squishy_volumes_crystal_grid = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Crystal Grid"
        )

        squishy_volumes_crystal_grid.color_tag = "NONE"
        squishy_volumes_crystal_grid.description = ""
        squishy_volumes_crystal_grid.default_group_node_width = 140

        # squishy_volumes_crystal_grid interface
        # Socket Instances
        instances_socket_1 = squishy_volumes_crystal_grid.interface.new_socket(
            name="Instances", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        instances_socket_1.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_3 = squishy_volumes_crystal_grid.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_3.attribute_domain = "POINT"
        geometry_socket_3.description = "Points to instance on"

        # Socket Scale
        scale_socket_1 = squishy_volumes_crystal_grid.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket_1.default_value = 0.5
        scale_socket_1.min_value = -10000.0
        scale_socket_1.max_value = 10000.0
        scale_socket_1.subtype = "NONE"
        scale_socket_1.attribute_domain = "POINT"

        # Socket Grid Node Size
        grid_node_size_socket = squishy_volumes_crystal_grid.interface.new_socket(
            name="Grid Node Size", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        grid_node_size_socket.default_value = 0.5
        grid_node_size_socket.min_value = -10000.0
        grid_node_size_socket.max_value = 10000.0
        grid_node_size_socket.subtype = "NONE"
        grid_node_size_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_crystal_grid nodes
        # node Group Output
        group_output_2 = squishy_volumes_crystal_grid.nodes.new("NodeGroupOutput")
        group_output_2.name = "Group Output"
        group_output_2.is_active_output = True

        # node Group Input
        group_input_2 = squishy_volumes_crystal_grid.nodes.new("NodeGroupInput")
        group_input_2.name = "Group Input"

        # node UV Sphere
        uv_sphere = squishy_volumes_crystal_grid.nodes.new("GeometryNodeMeshUVSphere")
        uv_sphere.name = "UV Sphere"
        # Segments
        uv_sphere.inputs[0].default_value = 4
        # Rings
        uv_sphere.inputs[1].default_value = 2

        # node Math.003
        math_003 = squishy_volumes_crystal_grid.nodes.new("ShaderNodeMath")
        math_003.name = "Math.003"
        math_003.operation = "MULTIPLY"
        math_003.use_clamp = False
        # Value_001
        math_003.inputs[1].default_value = 0.25

        # node Math.004
        math_004 = squishy_volumes_crystal_grid.nodes.new("ShaderNodeMath")
        math_004.name = "Math.004"
        math_004.operation = "MULTIPLY"
        math_004.use_clamp = False

        # node Instance on Points
        instance_on_points_1 = squishy_volumes_crystal_grid.nodes.new(
            "GeometryNodeInstanceOnPoints"
        )
        instance_on_points_1.name = "Instance on Points"
        # Selection
        instance_on_points_1.inputs[1].default_value = True
        # Pick Instance
        instance_on_points_1.inputs[3].default_value = False
        # Instance Index
        instance_on_points_1.inputs[4].default_value = 0
        # Rotation
        instance_on_points_1.inputs[5].default_value = (0.0, 0.0, 0.0)
        # Scale
        instance_on_points_1.inputs[6].default_value = (1.0, 1.0, 1.0)

        # Set locations
        group_output_2.location = (400.0, 0.0)
        group_input_2.location = (-480.0, -20.0)
        uv_sphere.location = (80.0, -80.0)
        math_003.location = (-100.0, -80.0)
        math_004.location = (-280.0, -80.0)
        instance_on_points_1.location = (240.0, 0.0)

        # Set dimensions
        group_output_2.width, group_output_2.height = 140.0, 100.0
        group_input_2.width, group_input_2.height = 140.0, 100.0
        uv_sphere.width, uv_sphere.height = 140.0, 100.0
        math_003.width, math_003.height = 140.0, 100.0
        math_004.width, math_004.height = 140.0, 100.0
        instance_on_points_1.width, instance_on_points_1.height = 140.0, 100.0

        # initialize squishy_volumes_crystal_grid links
        # math_003.Value -> uv_sphere.Radius
        squishy_volumes_crystal_grid.links.new(math_003.outputs[0], uv_sphere.inputs[2])
        # math_004.Value -> math_003.Value
        squishy_volumes_crystal_grid.links.new(math_004.outputs[0], math_003.inputs[0])
        # uv_sphere.Mesh -> instance_on_points_1.Instance
        squishy_volumes_crystal_grid.links.new(
            uv_sphere.outputs[0], instance_on_points_1.inputs[2]
        )
        # group_input_2.Scale -> math_004.Value
        squishy_volumes_crystal_grid.links.new(
            group_input_2.outputs[1], math_004.inputs[0]
        )
        # group_input_2.Geometry -> instance_on_points_1.Points
        squishy_volumes_crystal_grid.links.new(
            group_input_2.outputs[0], instance_on_points_1.inputs[0]
        )
        # group_input_2.Grid Node Size -> math_004.Value
        squishy_volumes_crystal_grid.links.new(
            group_input_2.outputs[2], math_004.inputs[1]
        )
        # instance_on_points_1.Instances -> group_output_2.Instances
        squishy_volumes_crystal_grid.links.new(
            instance_on_points_1.outputs[0], group_output_2.inputs[0]
        )
        return squishy_volumes_crystal_grid

    squishy_volumes_crystal_grid = squishy_volumes_crystal_grid_node_group()

    # initialize squishy_volumes_read_distance node group
    def squishy_volumes_read_distance_node_group():
        squishy_volumes_read_distance = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Read Distance"
        )

        squishy_volumes_read_distance.color_tag = "NONE"
        squishy_volumes_read_distance.description = ""
        squishy_volumes_read_distance.default_group_node_width = 140

        # squishy_volumes_read_distance interface
        # Socket Distance
        distance_socket = squishy_volumes_read_distance.interface.new_socket(
            name="Distance", in_out="OUTPUT", socket_type="NodeSocketFloat"
        )
        distance_socket.default_value = 0.0
        distance_socket.min_value = -3.4028234663852886e38
        distance_socket.max_value = 3.4028234663852886e38
        distance_socket.subtype = "NONE"
        distance_socket.attribute_domain = "POINT"

        # Socket Normal
        normal_socket = squishy_volumes_read_distance.interface.new_socket(
            name="Normal", in_out="OUTPUT", socket_type="NodeSocketVector"
        )
        normal_socket.default_value = (0.0, 0.0, 0.0)
        normal_socket.min_value = -3.4028234663852886e38
        normal_socket.max_value = 3.4028234663852886e38
        normal_socket.subtype = "NONE"
        normal_socket.attribute_domain = "POINT"

        # Socket Collider Idx
        collider_idx_socket = squishy_volumes_read_distance.interface.new_socket(
            name="Collider Idx", in_out="INPUT", socket_type="NodeSocketInt"
        )
        collider_idx_socket.default_value = 0
        collider_idx_socket.min_value = -2147483648
        collider_idx_socket.max_value = 2147483647
        collider_idx_socket.subtype = "NONE"
        collider_idx_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_read_distance nodes
        # node Group Output
        group_output_3 = squishy_volumes_read_distance.nodes.new("NodeGroupOutput")
        group_output_3.name = "Group Output"
        group_output_3.is_active_output = True

        # node Group Input
        group_input_3 = squishy_volumes_read_distance.nodes.new("NodeGroupInput")
        group_input_3.name = "Group Input"

        # node Join Strings
        join_strings = squishy_volumes_read_distance.nodes.new("GeometryNodeStringJoin")
        join_strings.name = "Join Strings"
        # Delimiter
        join_strings.inputs[0].default_value = "_"

        # node String
        string = squishy_volumes_read_distance.nodes.new("FunctionNodeInputString")
        string.name = "String"
        string.string = SQUISHY_VOLUMES_DISTANCE

        # node Named Attribute
        named_attribute = squishy_volumes_read_distance.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "FLOAT"

        # node Join Strings.001
        join_strings_001 = squishy_volumes_read_distance.nodes.new(
            "GeometryNodeStringJoin"
        )
        join_strings_001.name = "Join Strings.001"
        # Delimiter
        join_strings_001.inputs[0].default_value = "_"

        # node String.001
        string_001 = squishy_volumes_read_distance.nodes.new("FunctionNodeInputString")
        string_001.name = "String.001"
        string_001.string = SQUISHY_VOLUMES_NORMAL

        # node Named Attribute.001
        named_attribute_001 = squishy_volumes_read_distance.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_001.name = "Named Attribute.001"
        named_attribute_001.data_type = "FLOAT_VECTOR"

        # node Value to String
        value_to_string = squishy_volumes_read_distance.nodes.new(
            "FunctionNodeValueToString"
        )
        value_to_string.name = "Value to String"
        value_to_string.data_type = "INT"

        # Set locations
        group_output_3.location = (480.0, 0.0)
        group_input_3.location = (-480.0, 20.0)
        join_strings.location = (120.0, 100.0)
        string.location = (-100.0, 100.0)
        named_attribute.location = (280.0, 100.0)
        join_strings_001.location = (120.0, -60.0)
        string_001.location = (-100.0, -60.0)
        named_attribute_001.location = (280.0, -60.0)
        value_to_string.location = (-320.0, 40.0)

        # Set dimensions
        group_output_3.width, group_output_3.height = 140.0, 100.0
        group_input_3.width, group_input_3.height = 140.0, 100.0
        join_strings.width, join_strings.height = 140.0, 100.0
        string.width, string.height = 184.265625, 100.0
        named_attribute.width, named_attribute.height = 140.0, 100.0
        join_strings_001.width, join_strings_001.height = 140.0, 100.0
        string_001.width, string_001.height = 185.3314208984375, 100.0
        named_attribute_001.width, named_attribute_001.height = 140.0, 100.0
        value_to_string.width, value_to_string.height = 185.0560302734375, 100.0

        # initialize squishy_volumes_read_distance links
        # join_strings_001.String -> named_attribute_001.Name
        squishy_volumes_read_distance.links.new(
            join_strings_001.outputs[0], named_attribute_001.inputs[0]
        )
        # value_to_string.String -> join_strings.Strings
        squishy_volumes_read_distance.links.new(
            value_to_string.outputs[0], join_strings.inputs[1]
        )
        # value_to_string.String -> join_strings_001.Strings
        squishy_volumes_read_distance.links.new(
            value_to_string.outputs[0], join_strings_001.inputs[1]
        )
        # join_strings.String -> named_attribute.Name
        squishy_volumes_read_distance.links.new(
            join_strings.outputs[0], named_attribute.inputs[0]
        )
        # group_input_3.Collider Idx -> value_to_string.Value
        squishy_volumes_read_distance.links.new(
            group_input_3.outputs[0], value_to_string.inputs[0]
        )
        # named_attribute.Attribute -> group_output_3.Distance
        squishy_volumes_read_distance.links.new(
            named_attribute.outputs[0], group_output_3.inputs[0]
        )
        # named_attribute_001.Attribute -> group_output_3.Normal
        squishy_volumes_read_distance.links.new(
            named_attribute_001.outputs[0], group_output_3.inputs[1]
        )
        # string_001.String -> join_strings_001.Strings
        squishy_volumes_read_distance.links.new(
            string_001.outputs[0], join_strings_001.inputs[1]
        )
        # string.String -> join_strings.Strings
        squishy_volumes_read_distance.links.new(
            string.outputs[0], join_strings.inputs[1]
        )
        return squishy_volumes_read_distance

    squishy_volumes_read_distance = squishy_volumes_read_distance_node_group()

    # initialize squishy_volumes_grid_distance node group
    def squishy_volumes_grid_distance_node_group():
        squishy_volumes_grid_distance = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Grid Distance"
        )

        squishy_volumes_grid_distance.color_tag = "NONE"
        squishy_volumes_grid_distance.description = ""
        squishy_volumes_grid_distance.default_group_node_width = 140

        squishy_volumes_grid_distance.is_modifier = True

        # squishy_volumes_grid_distance interface
        # Socket Geometry
        geometry_socket_4 = squishy_volumes_grid_distance.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_4.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_5 = squishy_volumes_grid_distance.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_5.attribute_domain = "POINT"

        # Socket Collider Idx
        collider_idx_socket_1 = squishy_volumes_grid_distance.interface.new_socket(
            name="Collider Idx", in_out="INPUT", socket_type="NodeSocketInt"
        )
        collider_idx_socket_1.default_value = 0
        collider_idx_socket_1.min_value = 0
        collider_idx_socket_1.max_value = 2147483647
        collider_idx_socket_1.subtype = "NONE"
        collider_idx_socket_1.attribute_domain = "POINT"

        # Socket Scale
        scale_socket_2 = squishy_volumes_grid_distance.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket_2.default_value = 1.0
        scale_socket_2.min_value = 0.0
        scale_socket_2.max_value = 10000.0
        scale_socket_2.subtype = "NONE"
        scale_socket_2.attribute_domain = "POINT"

        # Socket Normal Scale
        normal_scale_socket = squishy_volumes_grid_distance.interface.new_socket(
            name="Normal Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        normal_scale_socket.default_value = 1.0
        normal_scale_socket.min_value = 0.0
        normal_scale_socket.max_value = 3.4028234663852886e38
        normal_scale_socket.subtype = "NONE"
        normal_scale_socket.attribute_domain = "POINT"

        # Socket Grid Node Size
        grid_node_size_socket_1 = squishy_volumes_grid_distance.interface.new_socket(
            name="Grid Node Size", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        grid_node_size_socket_1.default_value = 0.0
        grid_node_size_socket_1.min_value = 0.0
        grid_node_size_socket_1.max_value = 3.4028234663852886e38
        grid_node_size_socket_1.subtype = "NONE"
        grid_node_size_socket_1.attribute_domain = "POINT"
        grid_node_size_socket_1.hide_in_modifier = True

        # initialize squishy_volumes_grid_distance nodes
        # node Group Output
        group_output_4 = squishy_volumes_grid_distance.nodes.new("NodeGroupOutput")
        group_output_4.name = "Group Output"
        group_output_4.is_active_output = True

        # node Group
        group = squishy_volumes_grid_distance.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = squishy_volumes_color_instance

        # node Math
        math_1 = squishy_volumes_grid_distance.nodes.new("ShaderNodeMath")
        math_1.name = "Math"
        math_1.operation = "ADD"
        math_1.use_clamp = False
        # Value_001
        math_1.inputs[1].default_value = 0.5

        # node Math.001
        math_001 = squishy_volumes_grid_distance.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.operation = "DIVIDE"
        math_001.use_clamp = False

        # node Math.006
        math_006 = squishy_volumes_grid_distance.nodes.new("ShaderNodeMath")
        math_006.name = "Math.006"
        math_006.operation = "MULTIPLY"
        math_006.use_clamp = False
        # Value_001
        math_006.inputs[1].default_value = 2.0

        # node Color Ramp
        color_ramp = squishy_volumes_grid_distance.nodes.new("ShaderNodeValToRGB")
        color_ramp.name = "Color Ramp"
        color_ramp.color_ramp.color_mode = "RGB"
        color_ramp.color_ramp.hue_interpolation = "CW"
        color_ramp.color_ramp.interpolation = "LINEAR"

        # initialize color ramp elements
        color_ramp.color_ramp.elements.remove(color_ramp.color_ramp.elements[0])
        color_ramp_cre_0 = color_ramp.color_ramp.elements[0]
        color_ramp_cre_0.position = 0.0
        color_ramp_cre_0.alpha = 1.0
        color_ramp_cre_0.color = (1.0, 0.0, 0.0, 1.0)

        color_ramp_cre_1 = color_ramp.color_ramp.elements.new(0.5)
        color_ramp_cre_1.alpha = 1.0
        color_ramp_cre_1.color = (0.0, 0.0, 0.0, 1.0)

        color_ramp_cre_2 = color_ramp.color_ramp.elements.new(1.0)
        color_ramp_cre_2.alpha = 1.0
        color_ramp_cre_2.color = (0.0, 0.0, 1.0, 1.0)

        # node Group.002
        group_002 = squishy_volumes_grid_distance.nodes.new("GeometryNodeGroup")
        group_002.name = "Group.002"
        group_002.node_tree = squishy_volumes_vector

        # node Join Geometry
        join_geometry = squishy_volumes_grid_distance.nodes.new(
            "GeometryNodeJoinGeometry"
        )
        join_geometry.name = "Join Geometry"

        # node Delete Geometry
        delete_geometry = squishy_volumes_grid_distance.nodes.new(
            "GeometryNodeDeleteGeometry"
        )
        delete_geometry.name = "Delete Geometry"
        delete_geometry.domain = "POINT"
        delete_geometry.mode = "ALL"

        # node Math.002
        math_002 = squishy_volumes_grid_distance.nodes.new("ShaderNodeMath")
        math_002.name = "Math.002"
        math_002.operation = "GREATER_THAN"
        math_002.use_clamp = False
        # Value_001
        math_002.inputs[1].default_value = 1000000000.0

        # node Group.001
        group_001 = squishy_volumes_grid_distance.nodes.new("GeometryNodeGroup")
        group_001.name = "Group.001"
        group_001.node_tree = squishy_volumes_crystal_grid

        # node Group.003
        group_003 = squishy_volumes_grid_distance.nodes.new("GeometryNodeGroup")
        group_003.name = "Group.003"
        group_003.node_tree = squishy_volumes_read_distance

        # node Group Input.002
        group_input_002 = squishy_volumes_grid_distance.nodes.new("NodeGroupInput")
        group_input_002.name = "Group Input.002"

        # node Reroute
        reroute = squishy_volumes_grid_distance.nodes.new("NodeReroute")
        reroute.name = "Reroute"
        reroute.socket_idname = "NodeSocketGeometry"
        # node Reroute.001
        reroute_001 = squishy_volumes_grid_distance.nodes.new("NodeReroute")
        reroute_001.name = "Reroute.001"
        reroute_001.socket_idname = "NodeSocketFloat"
        # node Group Input.001
        group_input_001 = squishy_volumes_grid_distance.nodes.new("NodeGroupInput")
        group_input_001.name = "Group Input.001"

        # node Reroute.003
        reroute_003 = squishy_volumes_grid_distance.nodes.new("NodeReroute")
        reroute_003.name = "Reroute.003"
        reroute_003.socket_idname = "NodeSocketGeometry"

        # Set locations
        group_output_4.location = (760.0, 100.0)
        group.location = (260.0, 440.0)
        math_1.location = (-180.0, 300.0)
        math_001.location = (-340.0, 300.0)
        math_006.location = (-540.0, 380.0)
        color_ramp.location = (-20.0, 340.0)
        group_002.location = (-1060.0, 80.0)
        join_geometry.location = (600.0, 100.0)
        delete_geometry.location = (-1300.0, 460.0)
        math_002.location = (-1460.0, 360.0)
        group_001.location = (-460.0, 580.0)
        group_003.location = (-1740.0, 220.0)
        group_input_002.location = (-860.0, 460.0)
        reroute.location = (-1000.0, 480.0)
        reroute_001.location = (-1720.0, -40.0)
        group_input_001.location = (-1980.0, 240.0)
        reroute_003.location = (-1560.0, 380.0)

        # Set dimensions
        group_output_4.width, group_output_4.height = 140.0, 100.0
        group.width, group.height = 279.7696533203125, 100.0
        math_1.width, math_1.height = 140.0, 100.0
        math_001.width, math_001.height = 140.0, 100.0
        math_006.width, math_006.height = 140.0, 100.0
        color_ramp.width, color_ramp.height = 240.0, 100.0
        group_002.width, group_002.height = 230.84912109375, 100.0
        join_geometry.width, join_geometry.height = 140.0, 100.0
        delete_geometry.width, delete_geometry.height = 140.0, 100.0
        math_002.width, math_002.height = 140.0, 100.0
        group_001.width, group_001.height = 300.0, 100.0
        group_003.width, group_003.height = 260.0, 100.0
        group_input_002.width, group_input_002.height = 140.0, 100.0
        reroute.width, reroute.height = 10.0, 100.0
        reroute_001.width, reroute_001.height = 10.0, 100.0
        group_input_001.width, group_input_001.height = 140.0, 100.0
        reroute_003.width, reroute_003.height = 10.0, 100.0

        # initialize squishy_volumes_grid_distance links
        # join_geometry.Geometry -> group_output_4.Geometry
        squishy_volumes_grid_distance.links.new(
            join_geometry.outputs[0], group_output_4.inputs[0]
        )
        # math_001.Value -> math_1.Value
        squishy_volumes_grid_distance.links.new(math_001.outputs[0], math_1.inputs[0])
        # math_006.Value -> math_001.Value
        squishy_volumes_grid_distance.links.new(math_006.outputs[0], math_001.inputs[1])
        # group_002.Instances -> join_geometry.Geometry
        squishy_volumes_grid_distance.links.new(
            group_002.outputs[0], join_geometry.inputs[0]
        )
        # math_002.Value -> delete_geometry.Selection
        squishy_volumes_grid_distance.links.new(
            math_002.outputs[0], delete_geometry.inputs[1]
        )
        # math_1.Value -> color_ramp.Fac
        squishy_volumes_grid_distance.links.new(math_1.outputs[0], color_ramp.inputs[0])
        # group_001.Instances -> group.Geometry
        squishy_volumes_grid_distance.links.new(group_001.outputs[0], group.inputs[0])
        # color_ramp.Color -> group.Instance Color
        squishy_volumes_grid_distance.links.new(color_ramp.outputs[0], group.inputs[1])
        # reroute.Output -> group_001.Geometry
        squishy_volumes_grid_distance.links.new(reroute.outputs[0], group_001.inputs[0])
        # group_003.Distance -> math_002.Value
        squishy_volumes_grid_distance.links.new(
            group_003.outputs[0], math_002.inputs[0]
        )
        # group_input_002.Scale -> group_001.Scale
        squishy_volumes_grid_distance.links.new(
            group_input_002.outputs[2], group_001.inputs[1]
        )
        # group_input_002.Grid Node Size -> group_001.Grid Node Size
        squishy_volumes_grid_distance.links.new(
            group_input_002.outputs[4], group_001.inputs[2]
        )
        # delete_geometry.Geometry -> reroute.Input
        squishy_volumes_grid_distance.links.new(
            delete_geometry.outputs[0], reroute.inputs[0]
        )
        # reroute_001.Output -> group_002.Scale
        squishy_volumes_grid_distance.links.new(
            reroute_001.outputs[0], group_002.inputs[2]
        )
        # reroute_003.Output -> delete_geometry.Geometry
        squishy_volumes_grid_distance.links.new(
            reroute_003.outputs[0], delete_geometry.inputs[0]
        )
        # group_input_001.Collider Idx -> group_003.Collider Idx
        squishy_volumes_grid_distance.links.new(
            group_input_001.outputs[1], group_003.inputs[0]
        )
        # group_input_001.Normal Scale -> reroute_001.Input
        squishy_volumes_grid_distance.links.new(
            group_input_001.outputs[3], reroute_001.inputs[0]
        )
        # group_003.Normal -> group_002.Vector
        squishy_volumes_grid_distance.links.new(
            group_003.outputs[1], group_002.inputs[1]
        )
        # group_input_001.Geometry -> reroute_003.Input
        squishy_volumes_grid_distance.links.new(
            group_input_001.outputs[0], reroute_003.inputs[0]
        )
        # group_input_002.Grid Node Size -> math_006.Value
        squishy_volumes_grid_distance.links.new(
            group_input_002.outputs[4], math_006.inputs[0]
        )
        # group_003.Distance -> math_001.Value
        squishy_volumes_grid_distance.links.new(
            group_003.outputs[0], math_001.inputs[0]
        )
        # delete_geometry.Geometry -> group_002.Geometry
        squishy_volumes_grid_distance.links.new(
            delete_geometry.outputs[0], group_002.inputs[0]
        )
        # group.Geometry -> join_geometry.Geometry
        squishy_volumes_grid_distance.links.new(
            group.outputs[0], join_geometry.inputs[0]
        )
        return squishy_volumes_grid_distance

    return squishy_volumes_grid_distance_node_group()
