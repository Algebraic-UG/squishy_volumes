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

from ..magic_consts import SQUISHY_VOLUMES_NORMAL, SQUISHY_VOLUMES_VELOCITY


def create_geometry_nodes_surface_samples():
    # initialize squishy_volumes_vector node group
    def squishy_volumes_vector_node_group():
        squishy_volumes_vector = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Vector"
        )

        squishy_volumes_vector.color_tag = "NONE"
        squishy_volumes_vector.description = ""
        squishy_volumes_vector.default_group_node_width = 140

        # squishy_volumes_vector interface
        # Socket Geometry
        geometry_socket_6 = squishy_volumes_vector.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_6.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_7 = squishy_volumes_vector.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_7.attribute_domain = "POINT"

        # Socket Vector
        vector_socket = squishy_volumes_vector.interface.new_socket(
            name="Vector", in_out="INPUT", socket_type="NodeSocketVector"
        )
        vector_socket.default_value = (0.0, 0.0, 0.0)
        vector_socket.min_value = -3.4028234663852886e38
        vector_socket.max_value = 3.4028234663852886e38
        vector_socket.subtype = "NONE"
        vector_socket.attribute_domain = "POINT"

        # Socket Scale
        scale_socket_1 = squishy_volumes_vector.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket_1.default_value = 0.0
        scale_socket_1.min_value = -3.4028234663852886e38
        scale_socket_1.max_value = 3.4028234663852886e38
        scale_socket_1.subtype = "NONE"
        scale_socket_1.attribute_domain = "POINT"

        # initialize squishy_volumes_vector nodes
        # node Group Input
        group_input_5 = squishy_volumes_vector.nodes.new("NodeGroupInput")
        group_input_5.name = "Group Input"

        # node Vector Math
        vector_math = squishy_volumes_vector.nodes.new("ShaderNodeVectorMath")
        vector_math.name = "Vector Math"
        vector_math.operation = "LENGTH"
        # Vector_001
        vector_math.inputs[1].default_value = (0.0, 0.0, 0.0)
        # Vector_002
        vector_math.inputs[2].default_value = (0.0, 0.0, 0.0)
        # Scale
        vector_math.inputs[3].default_value = 1.0

        # node Math
        math_3 = squishy_volumes_vector.nodes.new("ShaderNodeMath")
        math_3.name = "Math"
        math_3.operation = "MULTIPLY"
        math_3.use_clamp = False
        # Value_002
        math_3.inputs[2].default_value = 0.5

        # node Mesh Line
        mesh_line = squishy_volumes_vector.nodes.new("GeometryNodeMeshLine")
        mesh_line.name = "Mesh Line"
        mesh_line.count_mode = "TOTAL"
        mesh_line.mode = "OFFSET"
        # Count
        mesh_line.inputs[0].default_value = 2
        # Resolution
        mesh_line.inputs[1].default_value = 1.0
        # Start Location
        mesh_line.inputs[2].default_value = (0.0, 0.0, 0.0)
        # Offset
        mesh_line.inputs[3].default_value = (1.0, 0.0, 0.0)

        # node Instance on Points
        instance_on_points_1 = squishy_volumes_vector.nodes.new(
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

        # node Group Output
        group_output_5 = squishy_volumes_vector.nodes.new("NodeGroupOutput")
        group_output_5.name = "Group Output"
        group_output_5.is_active_output = True

        # Set locations
        group_input_5.location = (0.0, -300.0)
        vector_math.location = (300.0, 0.0)
        math_3.location = (600.0, 0.0)
        mesh_line.location = (600.0, -600.0)
        instance_on_points_1.location = (900.0, -300.0)
        align_rotation_to_vector.location = (900.0, -600.0)
        rotate_instances.location = (1200.0, -600.0)
        group_output_5.location = (1500.0, -600.0)

        # Set dimensions
        group_input_5.width, group_input_5.height = 140.0, 100.0
        vector_math.width, vector_math.height = 140.0, 100.0
        math_3.width, math_3.height = 140.0, 100.0
        mesh_line.width, mesh_line.height = 140.0, 100.0
        instance_on_points_1.width, instance_on_points_1.height = 140.0, 100.0
        align_rotation_to_vector.width, align_rotation_to_vector.height = 140.0, 100.0
        rotate_instances.width, rotate_instances.height = 140.0, 100.0
        group_output_5.width, group_output_5.height = 140.0, 100.0

        # initialize squishy_volumes_vector links
        # group_input_5.Vector -> vector_math.Vector
        squishy_volumes_vector.links.new(
            group_input_5.outputs[1], vector_math.inputs[0]
        )
        # vector_math.Value -> math_3.Value
        squishy_volumes_vector.links.new(vector_math.outputs[1], math_3.inputs[0])
        # group_input_5.Scale -> math_3.Value
        squishy_volumes_vector.links.new(group_input_5.outputs[2], math_3.inputs[1])
        # group_input_5.Geometry -> instance_on_points_1.Points
        squishy_volumes_vector.links.new(
            group_input_5.outputs[0], instance_on_points_1.inputs[0]
        )
        # math_3.Value -> instance_on_points_1.Scale
        squishy_volumes_vector.links.new(
            math_3.outputs[0], instance_on_points_1.inputs[6]
        )
        # mesh_line.Mesh -> instance_on_points_1.Instance
        squishy_volumes_vector.links.new(
            mesh_line.outputs[0], instance_on_points_1.inputs[2]
        )
        # group_input_5.Vector -> align_rotation_to_vector.Vector
        squishy_volumes_vector.links.new(
            group_input_5.outputs[1], align_rotation_to_vector.inputs[2]
        )
        # instance_on_points_1.Instances -> rotate_instances.Instances
        squishy_volumes_vector.links.new(
            instance_on_points_1.outputs[0], rotate_instances.inputs[0]
        )
        # align_rotation_to_vector.Rotation -> rotate_instances.Rotation
        squishy_volumes_vector.links.new(
            align_rotation_to_vector.outputs[0], rotate_instances.inputs[2]
        )
        # rotate_instances.Instances -> group_output_5.Geometry
        squishy_volumes_vector.links.new(
            rotate_instances.outputs[0], group_output_5.inputs[0]
        )
        return squishy_volumes_vector

    squishy_volumes_vector = squishy_volumes_vector_node_group()

    # initialize squishy_volumes_surface_samples node group
    def squishy_volumes_surface_samples_node_group():
        squishy_volumes_surface_samples = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Surface Samples"
        )

        squishy_volumes_surface_samples.color_tag = "NONE"
        squishy_volumes_surface_samples.description = ""
        squishy_volumes_surface_samples.default_group_node_width = 140

        squishy_volumes_surface_samples.is_modifier = True

        # squishy_volumes_surface_samples interface
        # Socket Geometry
        geometry_socket_1 = squishy_volumes_surface_samples.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_2 = squishy_volumes_surface_samples.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_2.attribute_domain = "POINT"

        # Socket Attribute
        attribute_socket = squishy_volumes_surface_samples.interface.new_socket(
            name="Attribute", in_out="INPUT", socket_type="NodeSocketMenu"
        )
        attribute_socket.attribute_domain = "POINT"

        # Socket Scale
        scale_socket_1 = squishy_volumes_surface_samples.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket_1.default_value = 1.0
        scale_socket_1.min_value = 0.0
        scale_socket_1.max_value = 10000.0
        scale_socket_1.subtype = "NONE"
        scale_socket_1.attribute_domain = "POINT"

        # initialize squishy_volumes_surface_samples nodes
        # node Group Input
        group_input_1 = squishy_volumes_surface_samples.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Group Output
        group_output_1 = squishy_volumes_surface_samples.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # node Group
        group = squishy_volumes_surface_samples.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = squishy_volumes_vector

        # node Named Attribute
        named_attribute = squishy_volumes_surface_samples.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "FLOAT_VECTOR"
        # Name
        named_attribute.inputs[0].default_value = SQUISHY_VOLUMES_NORMAL

        # node Join Geometry
        join_geometry = squishy_volumes_surface_samples.nodes.new(
            "GeometryNodeJoinGeometry"
        )
        join_geometry.name = "Join Geometry"

        # node Menu Switch
        menu_switch = squishy_volumes_surface_samples.nodes.new(
            "GeometryNodeMenuSwitch"
        )
        menu_switch.name = "Menu Switch"
        menu_switch.active_index = 1
        menu_switch.data_type = "VECTOR"
        menu_switch.enum_items.clear()
        menu_switch.enum_items.new("Normal")
        menu_switch.enum_items[0].description = ""
        menu_switch.enum_items.new("Velocity")
        menu_switch.enum_items[1].description = ""

        # node Named Attribute.001
        named_attribute_001 = squishy_volumes_surface_samples.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_001.name = "Named Attribute.001"
        named_attribute_001.data_type = "FLOAT_VECTOR"
        # Name
        named_attribute_001.inputs[0].default_value = SQUISHY_VOLUMES_VELOCITY

        # node Reroute
        reroute = squishy_volumes_surface_samples.nodes.new("NodeReroute")
        reroute.name = "Reroute"
        reroute.socket_idname = "NodeSocketGeometry"
        # node Reroute.001
        reroute_001 = squishy_volumes_surface_samples.nodes.new("NodeReroute")
        reroute_001.name = "Reroute.001"
        reroute_001.socket_idname = "NodeSocketFloat"
        # node Reroute.002
        reroute_002 = squishy_volumes_surface_samples.nodes.new("NodeReroute")
        reroute_002.name = "Reroute.002"
        reroute_002.socket_idname = "NodeSocketFloat"
        # node Reroute.003
        reroute_003 = squishy_volumes_surface_samples.nodes.new("NodeReroute")
        reroute_003.name = "Reroute.003"
        reroute_003.socket_idname = "NodeSocketGeometry"

        # Set locations
        group_input_1.location = (-620.0, -280.0)
        group_output_1.location = (840.0, -220.0)
        group.location = (320.0, -280.0)
        named_attribute.location = (-280.0, -200.0)
        join_geometry.location = (660.0, -220.0)
        menu_switch.location = (100.0, -260.0)
        named_attribute_001.location = (-280.0, -360.0)
        reroute.location = (260.0, -180.0)
        reroute_001.location = (260.0, -500.0)
        reroute_002.location = (-320.0, -500.0)
        reroute_003.location = (-320.0, -180.0)

        # Set dimensions
        group_input_1.width, group_input_1.height = 140.0, 100.0
        group_output_1.width, group_output_1.height = 140.0, 100.0
        group.width, group.height = 308.26416015625, 100.0
        named_attribute.width, named_attribute.height = 253.92184448242188, 100.0
        join_geometry.width, join_geometry.height = 140.0, 100.0
        menu_switch.width, menu_switch.height = 140.0, 100.0
        named_attribute_001.width, named_attribute_001.height = (
            253.92184448242188,
            100.0,
        )
        reroute.width, reroute.height = 10.0, 100.0
        reroute_001.width, reroute_001.height = 10.0, 100.0
        reroute_002.width, reroute_002.height = 10.0, 100.0
        reroute_003.width, reroute_003.height = 10.0, 100.0

        # initialize squishy_volumes_surface_samples links
        # join_geometry.Geometry -> group_output_1.Geometry
        squishy_volumes_surface_samples.links.new(
            join_geometry.outputs[0], group_output_1.inputs[0]
        )
        # group_input_1.Attribute -> menu_switch.Menu
        squishy_volumes_surface_samples.links.new(
            group_input_1.outputs[1], menu_switch.inputs[0]
        )
        # named_attribute.Attribute -> menu_switch.Normal
        squishy_volumes_surface_samples.links.new(
            named_attribute.outputs[0], menu_switch.inputs[1]
        )
        # named_attribute_001.Attribute -> menu_switch.Velocity
        squishy_volumes_surface_samples.links.new(
            named_attribute_001.outputs[0], menu_switch.inputs[2]
        )
        # menu_switch.Output -> group.Vector
        squishy_volumes_surface_samples.links.new(
            menu_switch.outputs[0], group.inputs[1]
        )
        # group.Instances -> join_geometry.Geometry
        squishy_volumes_surface_samples.links.new(
            group.outputs[0], join_geometry.inputs[0]
        )
        # reroute_001.Output -> group.Scale
        squishy_volumes_surface_samples.links.new(
            reroute_001.outputs[0], group.inputs[2]
        )
        # reroute.Output -> group.Geometry
        squishy_volumes_surface_samples.links.new(reroute.outputs[0], group.inputs[0])
        # reroute_003.Output -> reroute.Input
        squishy_volumes_surface_samples.links.new(
            reroute_003.outputs[0], reroute.inputs[0]
        )
        # reroute_002.Output -> reroute_001.Input
        squishy_volumes_surface_samples.links.new(
            reroute_002.outputs[0], reroute_001.inputs[0]
        )
        # group_input_1.Scale -> reroute_002.Input
        squishy_volumes_surface_samples.links.new(
            group_input_1.outputs[2], reroute_002.inputs[0]
        )
        # group_input_1.Geometry -> reroute_003.Input
        squishy_volumes_surface_samples.links.new(
            group_input_1.outputs[0], reroute_003.inputs[0]
        )
        # reroute.Output -> join_geometry.Geometry
        squishy_volumes_surface_samples.links.new(
            reroute.outputs[0], join_geometry.inputs[0]
        )
        attribute_socket.default_value = "Normal"
        return squishy_volumes_surface_samples

    return squishy_volumes_surface_samples_node_group()
