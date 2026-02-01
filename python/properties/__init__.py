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

import bpy

from .squishy_volumes_object import Squishy_Volumes_Object
from .squishy_volumes_scene import (
    Squishy_Volumes_Scene,
    unsubscribe_from_selection,
    subscribe_to_selection,
)
from .squishy_volumes_simulation import Squishy_Volumes_Simulation
from .squishy_volumes_object_attributes import Squishy_Volumes_Optional_Attributes


classes = [
    Squishy_Volumes_Simulation,
    Squishy_Volumes_Scene,
    Squishy_Volumes_Optional_Attributes,
    Squishy_Volumes_Object,
]


def register_properties():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Object.squishy_volumes_object = bpy.props.PointerProperty(  # ty:ignore[unresolved-attribute]
        type=Squishy_Volumes_Object
    )
    bpy.types.Scene.squishy_volumes_scene = bpy.props.PointerProperty(  # ty:ignore[unresolved-attribute]
        type=Squishy_Volumes_Scene
    )
    subscribe_to_selection()

    print("Squishy Volumes properties registered.")


def unregister_properties():
    unsubscribe_from_selection()
    del bpy.types.Scene.squishy_volumes_scene  # ty:ignore[unresolved-attribute]
    del bpy.types.Object.squishy_volumes_object  # ty:ignore[unresolved-attribute]
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    print("Squishy Volumes properties unregistered.")
