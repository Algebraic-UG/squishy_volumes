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
    SQUISHY_VOLUMES_MASS,
    SQUISHY_VOLUMES_INSTANCE_COLOR,
    SQUISHY_VOLUMES_VELOCITY,
)

from .material_colored_instances import create_material_colored_instances


def create_geometry_nodes_grid_momentum():
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
        geometry_socket = squishy_volumes_color_instance.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = squishy_volumes_color_instance.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Instance Color
        instance_color_socket = squishy_volumes_color_instance.interface.new_socket(
            name="Instance Color", in_out="INPUT", socket_type="NodeSocketColor"
        )
        instance_color_socket.default_value = (0.0, 0.0, 0.0, 1.0)
        instance_color_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_color_instance nodes
        # node Group Input
        group_input = squishy_volumes_color_instance.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

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
        group_output = squishy_volumes_color_instance.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # Set locations
        group_input.location = (0.0, 0.0)
        store_named_attribute.location = (300.0, 0.0)
        set_material.location = (600.0, 0.0)
        group_output.location = (900.0, 0.0)

        # Set dimensions
        group_input.width, group_input.height = 140.0, 100.0
        store_named_attribute.width, store_named_attribute.height = 250.0, 100.0
        set_material.width, set_material.height = 250.0, 100.0
        group_output.width, group_output.height = 140.0, 100.0

        # initialize squishy_volumes_color_instance links
        # group_input.Geometry -> store_named_attribute.Geometry
        squishy_volumes_color_instance.links.new(
            group_input.outputs[0], store_named_attribute.inputs[0]
        )
        # group_input.Instance Color -> store_named_attribute.Value
        squishy_volumes_color_instance.links.new(
            group_input.outputs[1], store_named_attribute.inputs[3]
        )
        # store_named_attribute.Geometry -> set_material.Geometry
        squishy_volumes_color_instance.links.new(
            store_named_attribute.outputs[0], set_material.inputs[0]
        )
        # set_material.Geometry -> group_output.Geometry
        squishy_volumes_color_instance.links.new(
            set_material.outputs[0], group_output.inputs[0]
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

    # initialize squishy_volumes_grid_momentum node group
    def squishy_volumes_grid_momentum_node_group():
        squishy_volumes_grid_momentum = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Grid Momentum"
        )

        squishy_volumes_grid_momentum.color_tag = "NONE"
        squishy_volumes_grid_momentum.description = ""
        squishy_volumes_grid_momentum.default_group_node_width = 140

        squishy_volumes_grid_momentum.is_modifier = True

        # squishy_volumes_grid_momentum interface
        # Socket Geometry
        geometry_socket_4 = squishy_volumes_grid_momentum.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_4.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_5 = squishy_volumes_grid_momentum.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_5.attribute_domain = "POINT"

        # Socket Scale
        scale_socket_2 = squishy_volumes_grid_momentum.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket_2.default_value = 1.0
        scale_socket_2.min_value = 0.0
        scale_socket_2.max_value = 3.4028234663852886e38
        scale_socket_2.subtype = "NONE"
        scale_socket_2.attribute_domain = "POINT"

        # Socket Velocity Scale
        velocity_scale_socket = squishy_volumes_grid_momentum.interface.new_socket(
            name="Velocity Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        velocity_scale_socket.default_value = 1.0
        velocity_scale_socket.min_value = 0.0
        velocity_scale_socket.max_value = 3.4028234663852886e38
        velocity_scale_socket.subtype = "NONE"
        velocity_scale_socket.attribute_domain = "POINT"

        # Socket Mass Scale
        mass_scale_socket = squishy_volumes_grid_momentum.interface.new_socket(
            name="Mass Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        mass_scale_socket.default_value = 1.0
        mass_scale_socket.min_value = 0.0
        mass_scale_socket.max_value = 3.4028234663852886e38
        mass_scale_socket.subtype = "NONE"
        mass_scale_socket.attribute_domain = "POINT"

        # Socket Grid Node Size
        grid_node_size_socket_1 = squishy_volumes_grid_momentum.interface.new_socket(
            name="Grid Node Size", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        grid_node_size_socket_1.default_value = 0.5
        grid_node_size_socket_1.min_value = 0.0
        grid_node_size_socket_1.max_value = 3.4028234663852886e38
        grid_node_size_socket_1.subtype = "NONE"
        grid_node_size_socket_1.attribute_domain = "POINT"

        # initialize squishy_volumes_grid_momentum nodes
        # node Group Input
        group_input_3 = squishy_volumes_grid_momentum.nodes.new("NodeGroupInput")
        group_input_3.name = "Group Input"

        # node Group Output
        group_output_3 = squishy_volumes_grid_momentum.nodes.new("NodeGroupOutput")
        group_output_3.name = "Group Output"
        group_output_3.is_active_output = True

        # node Group
        group = squishy_volumes_grid_momentum.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = squishy_volumes_color_instance

        # node Named Attribute
        named_attribute = squishy_volumes_grid_momentum.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "FLOAT"
        # Name
        named_attribute.inputs[0].default_value = SQUISHY_VOLUMES_MASS

        # node Math.001
        math_001 = squishy_volumes_grid_momentum.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.operation = "MULTIPLY"
        math_001.use_clamp = False

        # node Group.001
        group_001 = squishy_volumes_grid_momentum.nodes.new("GeometryNodeGroup")
        group_001.name = "Group.001"
        group_001.node_tree = squishy_volumes_vector

        # node Join Geometry
        join_geometry = squishy_volumes_grid_momentum.nodes.new(
            "GeometryNodeJoinGeometry"
        )
        join_geometry.name = "Join Geometry"

        # node Named Attribute.001
        named_attribute_001 = squishy_volumes_grid_momentum.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_001.name = "Named Attribute.001"
        named_attribute_001.data_type = "FLOAT_VECTOR"
        # Name
        named_attribute_001.inputs[0].default_value = SQUISHY_VOLUMES_VELOCITY

        # node Delete Geometry
        delete_geometry = squishy_volumes_grid_momentum.nodes.new(
            "GeometryNodeDeleteGeometry"
        )
        delete_geometry.name = "Delete Geometry"
        delete_geometry.domain = "POINT"
        delete_geometry.mode = "ALL"

        # node Compare
        compare = squishy_volumes_grid_momentum.nodes.new("FunctionNodeCompare")
        compare.name = "Compare"
        compare.data_type = "FLOAT"
        compare.mode = "ELEMENT"
        compare.operation = "EQUAL"
        # B
        compare.inputs[1].default_value = 0.0
        # Epsilon
        compare.inputs[12].default_value = 0.0

        # node Mix
        mix = squishy_volumes_grid_momentum.nodes.new("ShaderNodeMix")
        mix.name = "Mix"
        mix.blend_type = "MIX"
        mix.clamp_factor = True
        mix.clamp_result = False
        mix.data_type = "RGBA"
        mix.factor_mode = "UNIFORM"
        # A_Color
        mix.inputs[6].default_value = (0.0, 0.0, 0.0, 1.0)
        # B_Color
        mix.inputs[7].default_value = (1.0, 1.0, 1.0, 1.0)

        # node Group Input.002
        group_input_002 = squishy_volumes_grid_momentum.nodes.new("NodeGroupInput")
        group_input_002.name = "Group Input.002"

        # node Named Attribute.002
        named_attribute_002 = squishy_volumes_grid_momentum.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_002.name = "Named Attribute.002"
        named_attribute_002.data_type = "FLOAT"
        # Name
        named_attribute_002.inputs[0].default_value = SQUISHY_VOLUMES_MASS

        # node Squishy Volumes Crystal Grid
        squishy_volumes_crystal_grid_1 = squishy_volumes_grid_momentum.nodes.new(
            "GeometryNodeGroup"
        )
        squishy_volumes_crystal_grid_1.name = "Squishy Volumes Crystal Grid"
        squishy_volumes_crystal_grid_1.node_tree = squishy_volumes_crystal_grid

        # node Group Input.001
        group_input_001 = squishy_volumes_grid_momentum.nodes.new("NodeGroupInput")
        group_input_001.name = "Group Input.001"

        # Set locations
        group_input_3.location = (-440.0, 440.0)
        group_output_3.location = (1440.0, 0.0)
        group.location = (880.0, 300.0)
        named_attribute.location = (-440.0, 260.0)
        math_001.location = (440.0, 220.0)
        group_001.location = (200.0, -40.0)
        join_geometry.location = (1260.0, 0.0)
        named_attribute_001.location = (-140.0, 120.0)
        delete_geometry.location = (-100.0, 420.0)
        compare.location = (-260.0, 320.0)
        mix.location = (640.0, 260.0)
        group_input_002.location = (180.0, 320.0)
        named_attribute_002.location = (180.0, 140.0)
        squishy_volumes_crystal_grid_1.location = (400.0, 440.0)
        group_input_001.location = (-80.0, -20.0)

        # Set dimensions
        group_input_3.width, group_input_3.height = 140.0, 100.0
        group_output_3.width, group_output_3.height = 140.0, 100.0
        group.width, group.height = 300.0, 100.0
        named_attribute.width, named_attribute.height = 160.0, 100.0
        math_001.width, math_001.height = 140.0, 100.0
        group_001.width, group_001.height = 224.57421875, 100.0
        join_geometry.width, join_geometry.height = 140.0, 100.0
        named_attribute_001.width, named_attribute_001.height = 200.0, 100.0
        delete_geometry.width, delete_geometry.height = 140.0, 100.0
        compare.width, compare.height = 140.0, 100.0
        mix.width, mix.height = 140.0, 100.0
        group_input_002.width, group_input_002.height = 140.0, 100.0
        named_attribute_002.width, named_attribute_002.height = 160.0, 100.0
        squishy_volumes_crystal_grid_1.width, squishy_volumes_crystal_grid_1.height = (
            260.0,
            100.0,
        )
        group_input_001.width, group_input_001.height = 140.0, 100.0

        # initialize squishy_volumes_grid_momentum links
        # join_geometry.Geometry -> group_output_3.Geometry
        squishy_volumes_grid_momentum.links.new(
            join_geometry.outputs[0], group_output_3.inputs[0]
        )
        # named_attribute_001.Attribute -> group_001.Vector
        squishy_volumes_grid_momentum.links.new(
            named_attribute_001.outputs[0], group_001.inputs[1]
        )
        # group_input_3.Geometry -> delete_geometry.Geometry
        squishy_volumes_grid_momentum.links.new(
            group_input_3.outputs[0], delete_geometry.inputs[0]
        )
        # delete_geometry.Geometry -> group_001.Geometry
        squishy_volumes_grid_momentum.links.new(
            delete_geometry.outputs[0], group_001.inputs[0]
        )
        # compare.Result -> delete_geometry.Selection
        squishy_volumes_grid_momentum.links.new(
            compare.outputs[0], delete_geometry.inputs[1]
        )
        # named_attribute.Attribute -> compare.A
        squishy_volumes_grid_momentum.links.new(
            named_attribute.outputs[0], compare.inputs[0]
        )
        # math_001.Value -> mix.Factor
        squishy_volumes_grid_momentum.links.new(math_001.outputs[0], mix.inputs[0])
        # mix.Result -> group.Instance Color
        squishy_volumes_grid_momentum.links.new(mix.outputs[2], group.inputs[1])
        # group_input_002.Scale -> squishy_volumes_crystal_grid_1.Scale
        squishy_volumes_grid_momentum.links.new(
            group_input_002.outputs[1], squishy_volumes_crystal_grid_1.inputs[1]
        )
        # group_input_002.Grid Node Size -> squishy_volumes_crystal_grid_1.Grid Node Size
        squishy_volumes_grid_momentum.links.new(
            group_input_002.outputs[4], squishy_volumes_crystal_grid_1.inputs[2]
        )
        # delete_geometry.Geometry -> squishy_volumes_crystal_grid_1.Geometry
        squishy_volumes_grid_momentum.links.new(
            delete_geometry.outputs[0], squishy_volumes_crystal_grid_1.inputs[0]
        )
        # squishy_volumes_crystal_grid_1.Instances -> group.Geometry
        squishy_volumes_grid_momentum.links.new(
            squishy_volumes_crystal_grid_1.outputs[0], group.inputs[0]
        )
        # named_attribute_002.Attribute -> math_001.Value
        squishy_volumes_grid_momentum.links.new(
            named_attribute_002.outputs[0], math_001.inputs[1]
        )
        # group_input_002.Mass Scale -> math_001.Value
        squishy_volumes_grid_momentum.links.new(
            group_input_002.outputs[3], math_001.inputs[0]
        )
        # group_001.Instances -> join_geometry.Geometry
        squishy_volumes_grid_momentum.links.new(
            group_001.outputs[0], join_geometry.inputs[0]
        )
        # group_input_001.Velocity Scale -> group_001.Scale
        squishy_volumes_grid_momentum.links.new(
            group_input_001.outputs[2], group_001.inputs[2]
        )
        # group.Geometry -> join_geometry.Geometry
        squishy_volumes_grid_momentum.links.new(
            group.outputs[0], join_geometry.inputs[0]
        )
        return squishy_volumes_grid_momentum

    return squishy_volumes_grid_momentum_node_group()
