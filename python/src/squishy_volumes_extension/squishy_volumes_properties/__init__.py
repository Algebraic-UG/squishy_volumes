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

from ..bridge import SimulationHandle
from ..preferences import get_print_debug_info

from .object import *
from .scene import *


def frame_to_load(props: Squishy_Volumes_Properties, frame: int) -> int | None:
    frame = frame - props.display_start_frame  # ty:ignore[unresolved-attribute]

    sim = SimulationHandle.get(uuid=props.uuid)
    if sim is None:
        return None

    simulated_frames = sim.available_frames()
    if simulated_frames < 1:
        return None
    max_frame = min(props.bake_frames, simulated_frames - 1)  # ty:ignore[unresolved-attribute]

    # clamping is more practical than not loading anything
    frame = max(0, min(max_frame, frame))

    return frame


classes = [
    Squishy_Volumes_Properties_Simulation,
    Squishy_Volumes_Properties_Scene,
    Squishy_Volumes_Properties_Input,
    Squishy_Volumes_Properties_Output,
    Squishy_Volumes_Properties,
]


def register_properties():
    for cls in classes:
        bpy.utils.register_class(cls)
    bpy.types.Object.squishy_volumes = bpy.props.PointerProperty(  # ty:ignore[unresolved-attribute]
        type=Squishy_Volumes_Properties
    )
    bpy.types.Scene.squishy_volumes = bpy.props.PointerProperty(  # ty:ignore[unresolved-attribute]
        type=Squishy_Volumes_Properties_Scene
    )
    subscribe_to_selection()

    if get_print_debug_info():
        print("Squishy Volumes properties registered.")


def unregister_properties():
    unsubscribe_from_selection()
    del bpy.types.Scene.squishy_volumes  # ty:ignore[unresolved-attribute]
    del bpy.types.Object.squishy_volumes  # ty:ignore[unresolved-attribute]
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
    if get_print_debug_info():
        print("Squishy Volumes properties unregistered.")
