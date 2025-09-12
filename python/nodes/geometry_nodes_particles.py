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
    SQUISHY_VOLUMES_COLLIDER_INSIDE,
    SQUISHY_VOLUMES_ELASTIC_ENERGY,
    SQUISHY_VOLUMES_INSTANCE_COLOR,
    SQUISHY_VOLUMES_TRANSFORM,
    SQUISHY_VOLUMES_VELOCITY,
)

from .material_colored_instances import create_material_colored_instances


def create_geometry_nodes_particles():
    material_colored_instances = create_material_colored_instances()

    # initialize squishy_volumes_deformed_cubes node group
    def squishy_volumes_deformed_cubes_node_group():
        squishy_volumes_deformed_cubes = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Deformed Cubes"
        )

        squishy_volumes_deformed_cubes.color_tag = "NONE"
        squishy_volumes_deformed_cubes.description = ""
        squishy_volumes_deformed_cubes.default_group_node_width = 140

        # squishy_volumes_deformed_cubes interface
        # Socket Geometry
        geometry_socket = squishy_volumes_deformed_cubes.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_1 = squishy_volumes_deformed_cubes.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_1.attribute_domain = "POINT"

        # Socket Scale
        scale_socket = squishy_volumes_deformed_cubes.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket.default_value = 0.0
        scale_socket.min_value = -3.4028234663852886e38
        scale_socket.max_value = 3.4028234663852886e38
        scale_socket.subtype = "NONE"
        scale_socket.attribute_domain = "POINT"

        # Socket Particle Size
        particle_size_socket = squishy_volumes_deformed_cubes.interface.new_socket(
            name="Particle Size", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        particle_size_socket.default_value = 0.0
        particle_size_socket.min_value = -3.4028234663852886e38
        particle_size_socket.max_value = 3.4028234663852886e38
        particle_size_socket.subtype = "NONE"
        particle_size_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_deformed_cubes nodes
        # node Group Input
        group_input = squishy_volumes_deformed_cubes.nodes.new("NodeGroupInput")
        group_input.name = "Group Input"

        # node Math
        math = squishy_volumes_deformed_cubes.nodes.new("ShaderNodeMath")
        math.name = "Math"
        math.operation = "MULTIPLY"
        math.use_clamp = False
        # Value_002
        math.inputs[2].default_value = 0.5

        # node Cube
        cube = squishy_volumes_deformed_cubes.nodes.new("GeometryNodeMeshCube")
        cube.name = "Cube"
        # Vertices X
        cube.inputs[1].default_value = 2
        # Vertices Y
        cube.inputs[2].default_value = 2
        # Vertices Z
        cube.inputs[3].default_value = 2

        # node Instance on Points
        instance_on_points = squishy_volumes_deformed_cubes.nodes.new(
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
        # Scale
        instance_on_points.inputs[6].default_value = (1.0, 1.0, 1.0)

        # node Named Attribute
        named_attribute = squishy_volumes_deformed_cubes.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute.name = "Named Attribute"
        named_attribute.data_type = "FLOAT4X4"
        # Name
        named_attribute.inputs[0].default_value = SQUISHY_VOLUMES_TRANSFORM

        # node Set Instance Transform
        set_instance_transform = squishy_volumes_deformed_cubes.nodes.new(
            "GeometryNodeSetInstanceTransform"
        )
        set_instance_transform.name = "Set Instance Transform"
        # Selection
        set_instance_transform.inputs[1].default_value = True

        # node Group Output
        group_output = squishy_volumes_deformed_cubes.nodes.new("NodeGroupOutput")
        group_output.name = "Group Output"
        group_output.is_active_output = True

        # Set locations
        group_input.location = (0.0, 0.0)
        math.location = (300.0, -300.0)
        cube.location = (600.0, -300.0)
        instance_on_points.location = (900.0, 0.0)
        named_attribute.location = (1200.0, -300.0)
        set_instance_transform.location = (1500.0, 0.0)
        group_output.location = (1800.0, 0.0)

        # Set dimensions
        group_input.width, group_input.height = 140.0, 100.0
        math.width, math.height = 140.0, 100.0
        cube.width, cube.height = 140.0, 100.0
        instance_on_points.width, instance_on_points.height = 140.0, 100.0
        named_attribute.width, named_attribute.height = 250.0, 100.0
        set_instance_transform.width, set_instance_transform.height = 160.0, 100.0
        group_output.width, group_output.height = 140.0, 100.0

        # initialize squishy_volumes_deformed_cubes links
        # group_input.Geometry -> instance_on_points.Points
        squishy_volumes_deformed_cubes.links.new(
            group_input.outputs[0], instance_on_points.inputs[0]
        )
        # group_input.Scale -> math.Value
        squishy_volumes_deformed_cubes.links.new(group_input.outputs[1], math.inputs[0])
        # group_input.Particle Size -> math.Value
        squishy_volumes_deformed_cubes.links.new(group_input.outputs[2], math.inputs[1])
        # math.Value -> cube.Size
        squishy_volumes_deformed_cubes.links.new(math.outputs[0], cube.inputs[0])
        # cube.Mesh -> instance_on_points.Instance
        squishy_volumes_deformed_cubes.links.new(
            cube.outputs[0], instance_on_points.inputs[2]
        )
        # instance_on_points.Instances -> set_instance_transform.Instances
        squishy_volumes_deformed_cubes.links.new(
            instance_on_points.outputs[0], set_instance_transform.inputs[0]
        )
        # named_attribute.Attribute -> set_instance_transform.Transform
        squishy_volumes_deformed_cubes.links.new(
            named_attribute.outputs[0], set_instance_transform.inputs[2]
        )
        # set_instance_transform.Instances -> group_output.Geometry
        squishy_volumes_deformed_cubes.links.new(
            set_instance_transform.outputs[0], group_output.inputs[0]
        )
        return squishy_volumes_deformed_cubes

    squishy_volumes_deformed_cubes = squishy_volumes_deformed_cubes_node_group()

    # initialize squishy_volumes_color_energy node group
    def squishy_volumes_color_energy_node_group():
        squishy_volumes_color_energy = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Color Energy"
        )

        squishy_volumes_color_energy.color_tag = "NONE"
        squishy_volumes_color_energy.description = ""
        squishy_volumes_color_energy.default_group_node_width = 140

        # squishy_volumes_color_energy interface
        # Socket Instance Color
        instance_color_socket = squishy_volumes_color_energy.interface.new_socket(
            name="Instance Color", in_out="OUTPUT", socket_type="NodeSocketColor"
        )
        instance_color_socket.default_value = (0.0, 0.0, 0.0, 1.0)
        instance_color_socket.attribute_domain = "INSTANCE"

        # Socket Divide by 10^x
        divide_by_10_x_socket = squishy_volumes_color_energy.interface.new_socket(
            name="Divide by 10^x", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        divide_by_10_x_socket.default_value = 0.0
        divide_by_10_x_socket.min_value = -3.4028234663852886e38
        divide_by_10_x_socket.max_value = 3.4028234663852886e38
        divide_by_10_x_socket.subtype = "NONE"
        divide_by_10_x_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_color_energy nodes
        # node Group Input
        group_input_1 = squishy_volumes_color_energy.nodes.new("NodeGroupInput")
        group_input_1.name = "Group Input"

        # node Named Attribute
        named_attribute_1 = squishy_volumes_color_energy.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_1.name = "Named Attribute"
        named_attribute_1.data_type = "FLOAT"
        # Name
        named_attribute_1.inputs[0].default_value = SQUISHY_VOLUMES_ELASTIC_ENERGY

        # node Math
        math_1 = squishy_volumes_color_energy.nodes.new("ShaderNodeMath")
        math_1.name = "Math"
        math_1.operation = "POWER"
        math_1.use_clamp = False
        # Value
        math_1.inputs[0].default_value = 10.0
        # Value_002
        math_1.inputs[2].default_value = 0.5

        # node Math.001
        math_001 = squishy_volumes_color_energy.nodes.new("ShaderNodeMath")
        math_001.name = "Math.001"
        math_001.operation = "DIVIDE"
        math_001.use_clamp = False
        # Value_002
        math_001.inputs[2].default_value = 0.5

        # node Color Ramp
        color_ramp = squishy_volumes_color_energy.nodes.new("ShaderNodeValToRGB")
        color_ramp.name = "Color Ramp"
        color_ramp.color_ramp.color_mode = "HSV"
        color_ramp.color_ramp.hue_interpolation = "CW"
        color_ramp.color_ramp.interpolation = "LINEAR"

        # initialize color ramp elements
        color_ramp.color_ramp.elements.remove(color_ramp.color_ramp.elements[0])
        color_ramp_cre_0 = color_ramp.color_ramp.elements[0]
        color_ramp_cre_0.position = 0.0
        color_ramp_cre_0.alpha = 1.0
        color_ramp_cre_0.color = (1.0, 0.0, 0.0, 1.0)

        color_ramp_cre_1 = color_ramp.color_ramp.elements.new(1.0)
        color_ramp_cre_1.alpha = 1.0
        color_ramp_cre_1.color = (0.800000011920929, 0.0, 1.0, 1.0)

        # node Group Output
        group_output_1 = squishy_volumes_color_energy.nodes.new("NodeGroupOutput")
        group_output_1.name = "Group Output"
        group_output_1.is_active_output = True

        # Set locations
        group_input_1.location = (0.0, 0.0)
        named_attribute_1.location = (300.0, 0.0)
        math_1.location = (300.0, -300.0)
        math_001.location = (600.0, 0.0)
        color_ramp.location = (900.0, 0.0)
        group_output_1.location = (1200.0, 0.0)

        # Set dimensions
        group_input_1.width, group_input_1.height = 140.0, 100.0
        named_attribute_1.width, named_attribute_1.height = 250.0, 100.0
        math_1.width, math_1.height = 140.0, 100.0
        math_001.width, math_001.height = 140.0, 100.0
        color_ramp.width, color_ramp.height = 240.0, 100.0
        group_output_1.width, group_output_1.height = 140.0, 100.0

        # initialize squishy_volumes_color_energy links
        # group_input_1.Divide by 10^x -> math_1.Value
        squishy_volumes_color_energy.links.new(
            group_input_1.outputs[0], math_1.inputs[1]
        )
        # named_attribute_1.Attribute -> math_001.Value
        squishy_volumes_color_energy.links.new(
            named_attribute_1.outputs[0], math_001.inputs[0]
        )
        # math_1.Value -> math_001.Value
        squishy_volumes_color_energy.links.new(math_1.outputs[0], math_001.inputs[1])
        # math_001.Value -> color_ramp.Fac
        squishy_volumes_color_energy.links.new(
            math_001.outputs[0], color_ramp.inputs[0]
        )
        # color_ramp.Color -> group_output_1.Instance Color
        squishy_volumes_color_energy.links.new(
            color_ramp.outputs[0], group_output_1.inputs[0]
        )
        return squishy_volumes_color_energy

    squishy_volumes_color_energy = squishy_volumes_color_energy_node_group()

    # initialize squishy_volumes_color_inside node group
    def squishy_volumes_color_inside_node_group():
        squishy_volumes_color_inside = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Color Inside"
        )

        squishy_volumes_color_inside.color_tag = "NONE"
        squishy_volumes_color_inside.description = ""
        squishy_volumes_color_inside.default_group_node_width = 140

        # squishy_volumes_color_inside interface
        # Socket Instance Color
        instance_color_socket_1 = squishy_volumes_color_inside.interface.new_socket(
            name="Instance Color", in_out="OUTPUT", socket_type="NodeSocketColor"
        )
        instance_color_socket_1.default_value = (0.0, 0.0, 0.0, 1.0)
        instance_color_socket_1.attribute_domain = "INSTANCE"

        # Socket Collider Idx
        collider_idx_socket = squishy_volumes_color_inside.interface.new_socket(
            name="Collider Idx", in_out="INPUT", socket_type="NodeSocketInt"
        )
        collider_idx_socket.default_value = 0
        collider_idx_socket.min_value = -2147483648
        collider_idx_socket.max_value = 2147483647
        collider_idx_socket.subtype = "NONE"
        collider_idx_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_color_inside nodes
        # node Group Input
        group_input_2 = squishy_volumes_color_inside.nodes.new("NodeGroupInput")
        group_input_2.name = "Group Input"

        # node String
        string = squishy_volumes_color_inside.nodes.new("FunctionNodeInputString")
        string.name = "String"
        string.string = SQUISHY_VOLUMES_COLLIDER_INSIDE

        # node Value to String
        value_to_string = squishy_volumes_color_inside.nodes.new(
            "FunctionNodeValueToString"
        )
        value_to_string.name = "Value to String"
        value_to_string.data_type = "INT"
        # Decimals
        value_to_string.inputs[1].default_value = 0

        # node Join Strings
        join_strings = squishy_volumes_color_inside.nodes.new("GeometryNodeStringJoin")
        join_strings.name = "Join Strings"
        # Delimiter
        join_strings.inputs[0].default_value = "_"

        # node Named Attribute
        named_attribute_2 = squishy_volumes_color_inside.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_2.name = "Named Attribute"
        named_attribute_2.data_type = "FLOAT"

        # node Math
        math_2 = squishy_volumes_color_inside.nodes.new("ShaderNodeMath")
        math_2.name = "Math"
        math_2.operation = "MULTIPLY"
        math_2.use_clamp = False
        # Value_001
        math_2.inputs[1].default_value = 0.5
        # Value_002
        math_2.inputs[2].default_value = 0.5

        # node Math.001
        math_001_1 = squishy_volumes_color_inside.nodes.new("ShaderNodeMath")
        math_001_1.name = "Math.001"
        math_001_1.operation = "ADD"
        math_001_1.use_clamp = False
        # Value_001
        math_001_1.inputs[1].default_value = 0.5
        # Value_002
        math_001_1.inputs[2].default_value = 0.5

        # node Color Ramp
        color_ramp_1 = squishy_volumes_color_inside.nodes.new("ShaderNodeValToRGB")
        color_ramp_1.name = "Color Ramp"
        color_ramp_1.color_ramp.color_mode = "HSV"
        color_ramp_1.color_ramp.hue_interpolation = "CW"
        color_ramp_1.color_ramp.interpolation = "LINEAR"

        # initialize color ramp elements
        color_ramp_1.color_ramp.elements.remove(color_ramp_1.color_ramp.elements[0])
        color_ramp_1_cre_0 = color_ramp_1.color_ramp.elements[0]
        color_ramp_1_cre_0.position = 0.0
        color_ramp_1_cre_0.alpha = 1.0
        color_ramp_1_cre_0.color = (1.0, 0.0, 0.0, 1.0)

        color_ramp_1_cre_1 = color_ramp_1.color_ramp.elements.new(1.0)
        color_ramp_1_cre_1.alpha = 1.0
        color_ramp_1_cre_1.color = (0.0, 0.0, 1.0, 1.0)

        # node Group Output
        group_output_2 = squishy_volumes_color_inside.nodes.new("NodeGroupOutput")
        group_output_2.name = "Group Output"
        group_output_2.is_active_output = True

        # Set locations
        group_input_2.location = (0.0, 0.0)
        string.location = (300.0, 0.0)
        value_to_string.location = (300.0, -300.0)
        join_strings.location = (600.0, 0.0)
        named_attribute_2.location = (900.0, 0.0)
        math_2.location = (1200.0, 0.0)
        math_001_1.location = (1500.0, 0.0)
        color_ramp_1.location = (1800.0, 0.0)
        group_output_2.location = (2100.0, 0.0)

        # Set dimensions
        group_input_2.width, group_input_2.height = 140.0, 100.0
        string.width, string.height = 250.0, 100.0
        value_to_string.width, value_to_string.height = 140.0, 100.0
        join_strings.width, join_strings.height = 140.0, 100.0
        named_attribute_2.width, named_attribute_2.height = 140.0, 100.0
        math_2.width, math_2.height = 140.0, 100.0
        math_001_1.width, math_001_1.height = 140.0, 100.0
        color_ramp_1.width, color_ramp_1.height = 240.0, 100.0
        group_output_2.width, group_output_2.height = 140.0, 100.0

        # initialize squishy_volumes_color_inside links
        # group_input_2.Collider Idx -> value_to_string.Value
        squishy_volumes_color_inside.links.new(
            group_input_2.outputs[0], value_to_string.inputs[0]
        )
        # value_to_string.String -> join_strings.Strings
        squishy_volumes_color_inside.links.new(
            value_to_string.outputs[0], join_strings.inputs[1]
        )
        # join_strings.String -> named_attribute_2.Name
        squishy_volumes_color_inside.links.new(
            join_strings.outputs[0], named_attribute_2.inputs[0]
        )
        # named_attribute_2.Attribute -> math_2.Value
        squishy_volumes_color_inside.links.new(
            named_attribute_2.outputs[0], math_2.inputs[0]
        )
        # math_2.Value -> math_001_1.Value
        squishy_volumes_color_inside.links.new(math_2.outputs[0], math_001_1.inputs[0])
        # math_001_1.Value -> color_ramp_1.Fac
        squishy_volumes_color_inside.links.new(
            math_001_1.outputs[0], color_ramp_1.inputs[0]
        )
        # color_ramp_1.Color -> group_output_2.Instance Color
        squishy_volumes_color_inside.links.new(
            color_ramp_1.outputs[0], group_output_2.inputs[0]
        )
        # string.String -> join_strings.Strings
        squishy_volumes_color_inside.links.new(
            string.outputs[0], join_strings.inputs[1]
        )
        return squishy_volumes_color_inside

    squishy_volumes_color_inside = squishy_volumes_color_inside_node_group()

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

    # initialize squishy_volumes_color_particle node group
    def squishy_volumes_color_particle_node_group():
        squishy_volumes_color_particle = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Color Particle"
        )

        squishy_volumes_color_particle.color_tag = "NONE"
        squishy_volumes_color_particle.description = ""
        squishy_volumes_color_particle.default_group_node_width = 140

        # squishy_volumes_color_particle interface
        # Socket Geometry
        geometry_socket_4 = squishy_volumes_color_particle.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_4.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_5 = squishy_volumes_color_particle.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_5.attribute_domain = "POINT"

        # Socket Coloring
        coloring_socket = squishy_volumes_color_particle.interface.new_socket(
            name="Coloring", in_out="INPUT", socket_type="NodeSocketMenu"
        )
        coloring_socket.attribute_domain = "POINT"

        # Socket Divide by 10^x
        divide_by_10_x_socket_1 = squishy_volumes_color_particle.interface.new_socket(
            name="Divide by 10^x", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        divide_by_10_x_socket_1.default_value = 0.0
        divide_by_10_x_socket_1.min_value = -3.4028234663852886e38
        divide_by_10_x_socket_1.max_value = 3.4028234663852886e38
        divide_by_10_x_socket_1.subtype = "NONE"
        divide_by_10_x_socket_1.attribute_domain = "POINT"

        # Socket Collider Idx
        collider_idx_socket_1 = squishy_volumes_color_particle.interface.new_socket(
            name="Collider Idx", in_out="INPUT", socket_type="NodeSocketInt"
        )
        collider_idx_socket_1.default_value = 0
        collider_idx_socket_1.min_value = -2147483648
        collider_idx_socket_1.max_value = 2147483647
        collider_idx_socket_1.subtype = "NONE"
        collider_idx_socket_1.attribute_domain = "POINT"

        # initialize squishy_volumes_color_particle nodes
        # node Group Input
        group_input_4 = squishy_volumes_color_particle.nodes.new("NodeGroupInput")
        group_input_4.name = "Group Input"

        # node Group
        group = squishy_volumes_color_particle.nodes.new("GeometryNodeGroup")
        group.name = "Group"
        group.node_tree = squishy_volumes_color_energy

        # node Group.001
        group_001 = squishy_volumes_color_particle.nodes.new("GeometryNodeGroup")
        group_001.name = "Group.001"
        group_001.node_tree = squishy_volumes_color_inside

        # node Menu Switch
        menu_switch = squishy_volumes_color_particle.nodes.new("GeometryNodeMenuSwitch")
        menu_switch.name = "Menu Switch"
        menu_switch.active_index = 1
        menu_switch.data_type = "RGBA"
        menu_switch.enum_items.clear()
        menu_switch.enum_items.new("Energy")
        menu_switch.enum_items[0].description = ""
        menu_switch.enum_items.new("Inside")
        menu_switch.enum_items[1].description = ""

        # node Group.002
        group_002 = squishy_volumes_color_particle.nodes.new("GeometryNodeGroup")
        group_002.name = "Group.002"
        group_002.node_tree = squishy_volumes_color_instance

        # node Group Output
        group_output_4 = squishy_volumes_color_particle.nodes.new("NodeGroupOutput")
        group_output_4.name = "Group Output"
        group_output_4.is_active_output = True

        # Set locations
        group_input_4.location = (0.0, -300.0)
        group.location = (300.0, -600.0)
        group_001.location = (300.0, -900.0)
        menu_switch.location = (600.0, -300.0)
        group_002.location = (900.0, 0.0)
        group_output_4.location = (1200.0, 0.0)

        # Set dimensions
        group_input_4.width, group_input_4.height = 140.0, 100.0
        group.width, group.height = 250.0, 100.0
        group_001.width, group_001.height = 250.0, 100.0
        menu_switch.width, menu_switch.height = 140.0, 100.0
        group_002.width, group_002.height = 250.0, 100.0
        group_output_4.width, group_output_4.height = 140.0, 100.0

        # initialize squishy_volumes_color_particle links
        # group_input_4.Geometry -> group_002.Geometry
        squishy_volumes_color_particle.links.new(
            group_input_4.outputs[0], group_002.inputs[0]
        )
        # group_input_4.Divide by 10^x -> group.Divide by 10^x
        squishy_volumes_color_particle.links.new(
            group_input_4.outputs[2], group.inputs[0]
        )
        # group_input_4.Collider Idx -> group_001.Collider Idx
        squishy_volumes_color_particle.links.new(
            group_input_4.outputs[3], group_001.inputs[0]
        )
        # group_input_4.Coloring -> menu_switch.Menu
        squishy_volumes_color_particle.links.new(
            group_input_4.outputs[1], menu_switch.inputs[0]
        )
        # group.Instance Color -> menu_switch.Energy
        squishy_volumes_color_particle.links.new(
            group.outputs[0], menu_switch.inputs[1]
        )
        # group_001.Instance Color -> menu_switch.Inside
        squishy_volumes_color_particle.links.new(
            group_001.outputs[0], menu_switch.inputs[2]
        )
        # menu_switch.Output -> group_002.Instance Color
        squishy_volumes_color_particle.links.new(
            menu_switch.outputs[0], group_002.inputs[1]
        )
        # group_002.Geometry -> group_output_4.Geometry
        squishy_volumes_color_particle.links.new(
            group_002.outputs[0], group_output_4.inputs[0]
        )
        return squishy_volumes_color_particle

    squishy_volumes_color_particle = squishy_volumes_color_particle_node_group()

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

    # initialize squishy_volumes_reconstruct node group
    def squishy_volumes_reconstruct_node_group():
        squishy_volumes_reconstruct = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Reconstruct"
        )

        squishy_volumes_reconstruct.color_tag = "NONE"
        squishy_volumes_reconstruct.description = ""
        squishy_volumes_reconstruct.default_group_node_width = 140

        # squishy_volumes_reconstruct interface
        # Socket Geometry
        geometry_socket_8 = squishy_volumes_reconstruct.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_8.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_9 = squishy_volumes_reconstruct.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_9.attribute_domain = "POINT"

        # Socket Particle Size
        particle_size_socket_1 = squishy_volumes_reconstruct.interface.new_socket(
            name="Particle Size", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        particle_size_socket_1.default_value = 0.0
        particle_size_socket_1.min_value = -3.4028234663852886e38
        particle_size_socket_1.max_value = 3.4028234663852886e38
        particle_size_socket_1.subtype = "NONE"
        particle_size_socket_1.attribute_domain = "POINT"

        # Socket Threshold
        threshold_socket = squishy_volumes_reconstruct.interface.new_socket(
            name="Threshold", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        threshold_socket.default_value = 0.0
        threshold_socket.min_value = -3.4028234663852886e38
        threshold_socket.max_value = 3.4028234663852886e38
        threshold_socket.subtype = "NONE"
        threshold_socket.attribute_domain = "POINT"

        # Socket Material
        material_socket = squishy_volumes_reconstruct.interface.new_socket(
            name="Material", in_out="INPUT", socket_type="NodeSocketMaterial"
        )
        material_socket.attribute_domain = "POINT"

        # initialize squishy_volumes_reconstruct nodes
        # node Group Input
        group_input_6 = squishy_volumes_reconstruct.nodes.new("NodeGroupInput")
        group_input_6.name = "Group Input"

        # node Math
        math_4 = squishy_volumes_reconstruct.nodes.new("ShaderNodeMath")
        math_4.name = "Math"
        math_4.operation = "MULTIPLY"
        math_4.use_clamp = False
        # Value_001
        math_4.inputs[1].default_value = 2.0
        # Value_002
        math_4.inputs[2].default_value = 0.5

        # node Points to Volume
        points_to_volume = squishy_volumes_reconstruct.nodes.new(
            "GeometryNodePointsToVolume"
        )
        points_to_volume.name = "Points to Volume"
        points_to_volume.resolution_mode = "VOXEL_SIZE"
        # Density
        points_to_volume.inputs[1].default_value = 1.0
        # Voxel Amount
        points_to_volume.inputs[3].default_value = 64.0

        # node Volume to Mesh
        volume_to_mesh = squishy_volumes_reconstruct.nodes.new(
            "GeometryNodeVolumeToMesh"
        )
        volume_to_mesh.name = "Volume to Mesh"
        volume_to_mesh.resolution_mode = "VOXEL_SIZE"
        # Voxel Amount
        volume_to_mesh.inputs[2].default_value = 64.0
        # Adaptivity
        volume_to_mesh.inputs[4].default_value = 0.0

        # node Set Material
        set_material_1 = squishy_volumes_reconstruct.nodes.new(
            "GeometryNodeSetMaterial"
        )
        set_material_1.name = "Set Material"
        # Selection
        set_material_1.inputs[1].default_value = True

        # node Group Output
        group_output_6 = squishy_volumes_reconstruct.nodes.new("NodeGroupOutput")
        group_output_6.name = "Group Output"
        group_output_6.is_active_output = True

        # Set locations
        group_input_6.location = (0.0, -300.0)
        math_4.location = (300.0, 0.0)
        points_to_volume.location = (600.0, 0.0)
        volume_to_mesh.location = (900.0, -300.0)
        set_material_1.location = (1200.0, -600.0)
        group_output_6.location = (1500.0, -600.0)

        # Set dimensions
        group_input_6.width, group_input_6.height = 140.0, 100.0
        math_4.width, math_4.height = 140.0, 100.0
        points_to_volume.width, points_to_volume.height = 170.0, 100.0
        volume_to_mesh.width, volume_to_mesh.height = 170.0, 100.0
        set_material_1.width, set_material_1.height = 140.0, 100.0
        group_output_6.width, group_output_6.height = 140.0, 100.0

        # initialize squishy_volumes_reconstruct links
        # group_input_6.Geometry -> points_to_volume.Points
        squishy_volumes_reconstruct.links.new(
            group_input_6.outputs[0], points_to_volume.inputs[0]
        )
        # group_input_6.Particle Size -> math_4.Value
        squishy_volumes_reconstruct.links.new(
            group_input_6.outputs[1], math_4.inputs[0]
        )
        # group_input_6.Particle Size -> points_to_volume.Voxel Size
        squishy_volumes_reconstruct.links.new(
            group_input_6.outputs[1], points_to_volume.inputs[2]
        )
        # math_4.Value -> points_to_volume.Radius
        squishy_volumes_reconstruct.links.new(
            math_4.outputs[0], points_to_volume.inputs[4]
        )
        # points_to_volume.Volume -> volume_to_mesh.Volume
        squishy_volumes_reconstruct.links.new(
            points_to_volume.outputs[0], volume_to_mesh.inputs[0]
        )
        # group_input_6.Particle Size -> volume_to_mesh.Voxel Size
        squishy_volumes_reconstruct.links.new(
            group_input_6.outputs[1], volume_to_mesh.inputs[1]
        )
        # group_input_6.Threshold -> volume_to_mesh.Threshold
        squishy_volumes_reconstruct.links.new(
            group_input_6.outputs[2], volume_to_mesh.inputs[3]
        )
        # volume_to_mesh.Mesh -> set_material_1.Geometry
        squishy_volumes_reconstruct.links.new(
            volume_to_mesh.outputs[0], set_material_1.inputs[0]
        )
        # group_input_6.Material -> set_material_1.Material
        squishy_volumes_reconstruct.links.new(
            group_input_6.outputs[3], set_material_1.inputs[2]
        )
        # set_material_1.Geometry -> group_output_6.Geometry
        squishy_volumes_reconstruct.links.new(
            set_material_1.outputs[0], group_output_6.inputs[0]
        )
        return squishy_volumes_reconstruct

    squishy_volumes_reconstruct = squishy_volumes_reconstruct_node_group()

    # initialize squishy_volumes_particle node group
    def squishy_volumes_particle_node_group():
        squishy_volumes_particle = bpy.data.node_groups.new(
            type="GeometryNodeTree", name="Squishy Volumes Particle"
        )

        squishy_volumes_particle.color_tag = "NONE"
        squishy_volumes_particle.description = ""
        squishy_volumes_particle.default_group_node_width = 140

        squishy_volumes_particle.is_modifier = True

        # squishy_volumes_particle interface
        # Socket Geometry
        geometry_socket_10 = squishy_volumes_particle.interface.new_socket(
            name="Geometry", in_out="OUTPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_10.attribute_domain = "POINT"

        # Socket Geometry
        geometry_socket_11 = squishy_volumes_particle.interface.new_socket(
            name="Geometry", in_out="INPUT", socket_type="NodeSocketGeometry"
        )
        geometry_socket_11.attribute_domain = "POINT"

        # Socket Display as
        display_as_socket = squishy_volumes_particle.interface.new_socket(
            name="Display as", in_out="INPUT", socket_type="NodeSocketMenu"
        )
        display_as_socket.attribute_domain = "POINT"

        # Socket Coloring
        coloring_socket_1 = squishy_volumes_particle.interface.new_socket(
            name="Coloring", in_out="INPUT", socket_type="NodeSocketMenu"
        )
        coloring_socket_1.attribute_domain = "POINT"

        # Socket Divide by 10^x
        divide_by_10_x_socket_2 = squishy_volumes_particle.interface.new_socket(
            name="Divide by 10^x", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        divide_by_10_x_socket_2.default_value = 3.0
        divide_by_10_x_socket_2.min_value = 0.0
        divide_by_10_x_socket_2.max_value = 3.4028234663852886e38
        divide_by_10_x_socket_2.subtype = "NONE"
        divide_by_10_x_socket_2.attribute_domain = "POINT"

        # Socket Collider Idx
        collider_idx_socket_2 = squishy_volumes_particle.interface.new_socket(
            name="Collider Idx", in_out="INPUT", socket_type="NodeSocketInt"
        )
        collider_idx_socket_2.default_value = 0
        collider_idx_socket_2.min_value = 0
        collider_idx_socket_2.max_value = 2147483647
        collider_idx_socket_2.subtype = "NONE"
        collider_idx_socket_2.attribute_domain = "POINT"

        # Socket Scale
        scale_socket_2 = squishy_volumes_particle.interface.new_socket(
            name="Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        scale_socket_2.default_value = 1.0
        scale_socket_2.min_value = 0.0
        scale_socket_2.max_value = 3.4028234663852886e38
        scale_socket_2.subtype = "NONE"
        scale_socket_2.attribute_domain = "POINT"

        # Socket Velocity Scale
        velocity_scale_socket = squishy_volumes_particle.interface.new_socket(
            name="Velocity Scale", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        velocity_scale_socket.default_value = 1.0
        velocity_scale_socket.min_value = 0.0
        velocity_scale_socket.max_value = 3.4028234663852886e38
        velocity_scale_socket.subtype = "NONE"
        velocity_scale_socket.attribute_domain = "POINT"

        # Socket Particle Size
        particle_size_socket_2 = squishy_volumes_particle.interface.new_socket(
            name="Particle Size", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        particle_size_socket_2.default_value = 0.25
        particle_size_socket_2.min_value = 0.0
        particle_size_socket_2.max_value = 3.4028234663852886e38
        particle_size_socket_2.subtype = "NONE"
        particle_size_socket_2.attribute_domain = "POINT"

        # Socket Threshold
        threshold_socket_1 = squishy_volumes_particle.interface.new_socket(
            name="Threshold", in_out="INPUT", socket_type="NodeSocketFloat"
        )
        threshold_socket_1.default_value = 0.5
        threshold_socket_1.min_value = 0.0
        threshold_socket_1.max_value = 3.4028234663852886e38
        threshold_socket_1.subtype = "NONE"
        threshold_socket_1.attribute_domain = "POINT"

        # Socket Material
        material_socket_1 = squishy_volumes_particle.interface.new_socket(
            name="Material", in_out="INPUT", socket_type="NodeSocketMaterial"
        )
        material_socket_1.attribute_domain = "POINT"

        # initialize squishy_volumes_particle nodes
        # node Group Input
        group_input_7 = squishy_volumes_particle.nodes.new("NodeGroupInput")
        group_input_7.name = "Group Input"

        # node Group
        group_1 = squishy_volumes_particle.nodes.new("GeometryNodeGroup")
        group_1.name = "Group"
        group_1.node_tree = squishy_volumes_deformed_cubes

        # node Group.001
        group_001_1 = squishy_volumes_particle.nodes.new("GeometryNodeGroup")
        group_001_1.name = "Group.001"
        group_001_1.node_tree = squishy_volumes_color_particle

        # node Named Attribute
        named_attribute_3 = squishy_volumes_particle.nodes.new(
            "GeometryNodeInputNamedAttribute"
        )
        named_attribute_3.name = "Named Attribute"
        named_attribute_3.data_type = "FLOAT_VECTOR"
        # Name
        named_attribute_3.inputs[0].default_value = SQUISHY_VOLUMES_VELOCITY

        # node Group.002
        group_002_1 = squishy_volumes_particle.nodes.new("GeometryNodeGroup")
        group_002_1.name = "Group.002"
        group_002_1.node_tree = squishy_volumes_vector

        # node Group.003
        group_003 = squishy_volumes_particle.nodes.new("GeometryNodeGroup")
        group_003.name = "Group.003"
        group_003.node_tree = squishy_volumes_reconstruct

        # node Join Geometry
        join_geometry = squishy_volumes_particle.nodes.new("GeometryNodeJoinGeometry")
        join_geometry.name = "Join Geometry"

        # node Menu Switch
        menu_switch_1 = squishy_volumes_particle.nodes.new("GeometryNodeMenuSwitch")
        menu_switch_1.name = "Menu Switch"
        menu_switch_1.active_index = 1
        menu_switch_1.data_type = "GEOMETRY"
        menu_switch_1.enum_items.clear()
        menu_switch_1.enum_items.new("Deformed Cubes")
        menu_switch_1.enum_items[0].description = ""
        menu_switch_1.enum_items.new("Reconstructed")
        menu_switch_1.enum_items[1].description = ""

        # node Group Output
        group_output_7 = squishy_volumes_particle.nodes.new("NodeGroupOutput")
        group_output_7.name = "Group Output"
        group_output_7.is_active_output = True

        # Set locations
        group_input_7.location = (0.0, -300.0)
        group_1.location = (300.0, 0.0)
        group_001_1.location = (600.0, 0.0)
        named_attribute_3.location = (300.0, -300.0)
        group_002_1.location = (600.0, -600.0)
        group_003.location = (600.0, -900.0)
        join_geometry.location = (900.0, -300.0)
        menu_switch_1.location = (1200.0, -600.0)
        group_output_7.location = (1500.0, -600.0)

        # Set dimensions
        group_input_7.width, group_input_7.height = 140.0, 100.0
        group_1.width, group_1.height = 250.0, 100.0
        group_001_1.width, group_001_1.height = 250.0, 100.0
        named_attribute_3.width, named_attribute_3.height = 250.0, 100.0
        group_002_1.width, group_002_1.height = 250.0, 100.0
        group_003.width, group_003.height = 250.0, 100.0
        join_geometry.width, join_geometry.height = 140.0, 100.0
        menu_switch_1.width, menu_switch_1.height = 140.0, 100.0
        group_output_7.width, group_output_7.height = 140.0, 100.0

        # initialize squishy_volumes_particle links
        # group_input_7.Geometry -> group_1.Geometry
        squishy_volumes_particle.links.new(group_input_7.outputs[0], group_1.inputs[0])
        # group_input_7.Geometry -> group_002_1.Geometry
        squishy_volumes_particle.links.new(
            group_input_7.outputs[0], group_002_1.inputs[0]
        )
        # group_input_7.Geometry -> group_003.Geometry
        squishy_volumes_particle.links.new(
            group_input_7.outputs[0], group_003.inputs[0]
        )
        # group_input_7.Display as -> menu_switch_1.Menu
        squishy_volumes_particle.links.new(
            group_input_7.outputs[1], menu_switch_1.inputs[0]
        )
        # group_input_7.Coloring -> group_001_1.Coloring
        squishy_volumes_particle.links.new(
            group_input_7.outputs[2], group_001_1.inputs[1]
        )
        # group_input_7.Divide by 10^x -> group_001_1.Divide by 10^x
        squishy_volumes_particle.links.new(
            group_input_7.outputs[3], group_001_1.inputs[2]
        )
        # group_input_7.Collider Idx -> group_001_1.Collider Idx
        squishy_volumes_particle.links.new(
            group_input_7.outputs[4], group_001_1.inputs[3]
        )
        # group_input_7.Scale -> group_1.Scale
        squishy_volumes_particle.links.new(group_input_7.outputs[5], group_1.inputs[1])
        # group_input_7.Velocity Scale -> group_002_1.Scale
        squishy_volumes_particle.links.new(
            group_input_7.outputs[6], group_002_1.inputs[2]
        )
        # group_input_7.Particle Size -> group_1.Particle Size
        squishy_volumes_particle.links.new(group_input_7.outputs[7], group_1.inputs[2])
        # group_input_7.Particle Size -> group_003.Particle Size
        squishy_volumes_particle.links.new(
            group_input_7.outputs[7], group_003.inputs[1]
        )
        # group_input_7.Threshold -> group_003.Threshold
        squishy_volumes_particle.links.new(
            group_input_7.outputs[8], group_003.inputs[2]
        )
        # group_input_7.Material -> group_003.Material
        squishy_volumes_particle.links.new(
            group_input_7.outputs[9], group_003.inputs[3]
        )
        # group_1.Geometry -> group_001_1.Geometry
        squishy_volumes_particle.links.new(group_1.outputs[0], group_001_1.inputs[0])
        # named_attribute_3.Attribute -> group_002_1.Vector
        squishy_volumes_particle.links.new(
            named_attribute_3.outputs[0], group_002_1.inputs[1]
        )
        # group_002_1.Geometry -> join_geometry.Geometry
        squishy_volumes_particle.links.new(
            group_002_1.outputs[0], join_geometry.inputs[0]
        )
        # join_geometry.Geometry -> menu_switch_1.Deformed Cubes
        squishy_volumes_particle.links.new(
            join_geometry.outputs[0], menu_switch_1.inputs[1]
        )
        # group_003.Geometry -> menu_switch_1.Reconstructed
        squishy_volumes_particle.links.new(
            group_003.outputs[0], menu_switch_1.inputs[2]
        )
        # menu_switch_1.Output -> group_output_7.Geometry
        squishy_volumes_particle.links.new(
            menu_switch_1.outputs[0], group_output_7.inputs[0]
        )
        # group_001_1.Geometry -> join_geometry.Geometry
        squishy_volumes_particle.links.new(
            group_001_1.outputs[0], join_geometry.inputs[0]
        )
        display_as_socket.default_value = "Deformed Cubes"
        coloring_socket_1.default_value = "Energy"
        return squishy_volumes_particle

    return squishy_volumes_particle_node_group()
