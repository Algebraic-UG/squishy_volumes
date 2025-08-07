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

import json
import numpy as np
import bpy

from .properties.blended_mpm_object_settings import (
    OBJECT_ENUM_COLLIDER,
    OBJECT_ENUM_FLUID,
    OBJECT_ENUM_SOLID,
)
from .properties.util import get_input_objects, get_simulation_specific_settings
from .util import array_to_base64, attribute_to_base64


def is_scripted(simulation, obj):
    obj_settings = get_simulation_specific_settings(simulation, obj)
    return obj_settings.object_enum == OBJECT_ENUM_COLLIDER


def create_setup_json(simulation):
    scene = bpy.context.scene

    input_objects = []
    serialized_vectors = {}

    for obj in get_input_objects(simulation):
        name = obj.name
        obj_settings = get_simulation_specific_settings(simulation, obj)

        scene.frame_set(simulation.capture_start_frame)

        obj_scale = obj.matrix_world.to_scale()
        if obj_scale.x < 0 or obj_scale.y < 0 or obj_scale.z < 0:
            raise RuntimeError(
                "negative scaling is not supported, please check '" + name + "'"
            )

        obj_position = obj.matrix_world.translation
        obj_orientation = obj.matrix_world.to_quaternion()

        scale = [
            obj_scale.x,
            obj_scale.y,
            obj_scale.z,
        ]
        position = [
            obj_position.x,
            obj_position.y,
            obj_position.z,
        ]
        orientation = [
            obj_orientation.x,
            obj_orientation.y,
            obj_orientation.z,
            obj_orientation.w,
        ]

        linear_velocity = [
            obj_settings.initial_linear_velocity[0],
            obj_settings.initial_linear_velocity[1],
            obj_settings.initial_linear_velocity[2],
        ]
        angular_velocity = [
            obj_settings.initial_angular_velocity[0],
            obj_settings.initial_angular_velocity[1],
            obj_settings.initial_angular_velocity[2],
        ]

        object_settings = None
        match obj_settings.object_enum:
            case e if e == OBJECT_ENUM_SOLID:
                object_settings = {
                    OBJECT_ENUM_SOLID: {
                        "density": obj_settings.density,
                        "youngs_modulus": obj_settings.youngs_modulus,
                        "poissons_ratio": obj_settings.poissons_ratio,
                        "dilation": obj_settings.dilation,
                        "randomness": obj_settings.randomness,
                    }
                }
            case e if e == OBJECT_ENUM_FLUID:
                object_settings = {
                    OBJECT_ENUM_FLUID: {
                        "density": obj_settings.density,
                        "exponent": obj_settings.exponent,
                        "bulk_modulus": obj_settings.bulk_modulus,
                        "dilation": obj_settings.dilation,
                        "randomness": obj_settings.randomness,
                    }
                }
            case e if e == OBJECT_ENUM_COLLIDER:
                object_settings = {
                    OBJECT_ENUM_COLLIDER: {
                        "sticky_factor": obj_settings.sticky_factor,
                        "friction_factor": obj_settings.friction_factor,
                    }
                }

        if is_scripted(simulation, obj):
            scripted_positions_array = np.empty(
                simulation.capture_frames * 3, dtype="float32"
            )
            scripted_orientations_array = np.empty(
                simulation.capture_frames * 4, dtype="float32"
            )

            initial_scale = None
            i = 0
            for frame in range(
                simulation.capture_start_frame,
                simulation.capture_start_frame + simulation.capture_frames,
            ):
                scene.frame_set(frame)

                obj_scale = obj.matrix_world.to_scale()
                if initial_scale:
                    if (obj_scale - initial_scale).length_squared > 1e-5:
                        raise RuntimeError(
                            "animated scaling is not supported, please check '"
                            + name
                            + "'"
                        )
                else:
                    initial_scale = obj_scale

                obj_position = obj.matrix_world.translation
                obj_orientation = obj.matrix_world.to_quaternion()

                scripted_positions_array[3 * i + 0] = obj_position.x
                scripted_positions_array[3 * i + 1] = obj_position.y
                scripted_positions_array[3 * i + 2] = obj_position.z

                scripted_orientations_array[4 * i + 0] = obj_orientation.x
                scripted_orientations_array[4 * i + 1] = obj_orientation.y
                scripted_orientations_array[4 * i + 2] = obj_orientation.z
                scripted_orientations_array[4 * i + 3] = obj_orientation.w

                i += 1
        else:
            scripted_positions_array = np.empty(0, dtype="float32")
            scripted_orientations_array = np.empty(0, dtype="float32")

        vertices = name + "_vertices"
        triangles = name + "_triangles"
        triangle_normals = name + "_triangle_normals"
        scripted_positions = name + "_scripted_positions"
        scripted_orientations = name + "_scripted_orientations"

        input_objects.append(
            {
                "object": {
                    "name": name,
                    "scale": scale,
                    "position": position,
                    "orientation": orientation,
                    "linear_velocity": linear_velocity,
                    "angular_velocity": angular_velocity,
                    "settings": object_settings,
                },
                "mesh_handles": {
                    "vertices": vertices,
                    "triangles": triangles,
                    "triangle_normals": triangle_normals,
                },
                "scripted_handles": {
                    "scripted_positions": scripted_positions,
                    "scripted_orientations": scripted_orientations,
                },
            }
        )

        serialized_vectors[vertices] = attribute_to_base64(
            obj.data.vertices, "co", "float32", 3
        )
        serialized_vectors[triangles] = attribute_to_base64(
            obj.data.loop_triangles, "vertices", "int32", 3
        )
        serialized_vectors[triangle_normals] = attribute_to_base64(
            obj.data.loop_triangles, "normal", "float32", 3
        )
        serialized_vectors[scripted_positions] = array_to_base64(
            scripted_positions_array
        )
        serialized_vectors[scripted_orientations] = array_to_base64(
            scripted_orientations_array
        )

    gravity = [
        simulation.to_cache.gravity[0],
        simulation.to_cache.gravity[1],
        simulation.to_cache.gravity[2],
    ]

    settings = {
        "grid_node_size": simulation.to_cache.grid_node_size,
        "particle_size": simulation.to_cache.particle_size,
        "frames_per_second": simulation.to_cache.frames_per_second,
        "gravity": gravity,
    }

    bulk_data = {
        "serialized_vectors": serialized_vectors,
    }

    return json.dumps(
        {
            "settings": settings,
            "objects": input_objects,
            "bulk_data": bulk_data,
        }
    )
