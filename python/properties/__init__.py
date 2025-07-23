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

import bpy

from .blended_mpm_collection import Blended_MPM_Collection
from .blended_mpm_object import Blended_MPM_Object
from .blended_mpm_object_settings import Blended_MPM_Object_Settings
from .blended_mpm_scene import Blended_MPM_Scene
from .blended_mpm_simulation import Blended_MPM_Simulation
from .blended_mpm_simulation_settings import Blended_MPM_Simulation_Settings
from .blended_mpm_object_attributes import Blended_MPM_Optional_Attributes


classes = [
    Blended_MPM_Simulation_Settings,
    Blended_MPM_Simulation,
    Blended_MPM_Scene,
    Blended_MPM_Collection,
    Blended_MPM_Object_Settings,
    Blended_MPM_Optional_Attributes,
    Blended_MPM_Object,
]


def register_properties():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Object.blended_mpm_object = bpy.props.PointerProperty(
        type=Blended_MPM_Object
    )
    bpy.types.Collection.blended_mpm_collection = bpy.props.PointerProperty(
        type=Blended_MPM_Collection
    )
    bpy.types.Scene.blended_mpm_scene = bpy.props.PointerProperty(
        type=Blended_MPM_Scene
    )

    print("Blended MPM properties registered.")


def unregister_properties():
    del bpy.types.Scene.blended_mpm_scene
    del bpy.types.Collection.blended_mpm_collection
    del bpy.types.Object.blended_mpm_object
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    print("Blended MPM properties unregistered.")
