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

import json
import time
import bpy

from .popup import with_popup
from .output import (
    sync_output,
)

from .preferences import get_print_debug_info
from .bridge import SimulationHandle
from .squishy_volumes_properties import (
    get_simulation_objects,
    get_output_objects_with_uuid,
    Squishy_Volumes_Properties,
    Squishy_Volumes_Properties_Simulation,
    INPUT_TYPE_COLLIDER,
    frame_to_load,
)


def sync(scene):
    for sim_obj in get_simulation_objects():
        sim_props = sim_obj.squishy_volumes
        if not sim_props.sync:  # ty:ignore[unresolved-attribute]
            # https://github.com/Algebraic-UG/squishy_volumes/issues/175
            for obj in get_output_objects_with_uuid(
                sim_props.uuid  # ty:ignore[unresolved-attribute]
            ):
                if obj.data is not None:
                    obj.data.update_tag()
            continue
        sim_handle = SimulationHandle.get(
            uuid=sim_props.uuid  # ty:ignore[unresolved-attribute]
        )
        if sim_handle is None:
            continue
        sync_simulation(
            sim_props,  # ty:ignore[unresolved-attribute]
            sim_handle,
            scene.frame_current,
        )


def sync_simulation(
    sim_props: Squishy_Volumes_Properties,
    sim_handle: SimulationHandle,
    frame: int,
):
    sim_props.has_loaded_frame = False  # ty:ignore[unresolved-attribute]

    frame = frame_to_load(sim_props, frame)  # ty:ignore[invalid-assignment]
    if frame is None:
        return

    sim_props.has_loaded_frame = True  # ty:ignore[unresolved-attribute]
    sim_props.loaded_frame = frame  # ty:ignore[unresolved-attribute]

    input_header = sim_handle.input_header()

    desynced_objs = []
    for output_obj in get_output_objects_with_uuid(sim_props.uuid):
        try:
            sync_output(sim_handle, output_obj, frame)
        except RuntimeError as e:
            desynced_objs.append((output_obj, e))

    if desynced_objs:
        for output_obj, _ in desynced_objs:
            output_obj.squishy_volumes.uuid = "broken"

        def raise_():
            message = """These output objects could not be synced and
have been decoupled from the output of the simulation.
(Most likely, the respective input object
is now incompatible or gone.)

"""
            for obj, e in desynced_objs:
                message += f"{obj.name}: {str(e)}"

            raise RuntimeError(message)

        with_popup(uuid=sim_props.uuid, f=raise_)


def frame_change_handler(scene):
    start = time.time()
    sync(scene)
    end = time.time()
    if get_print_debug_info():
        print("Squishy Volumes: sync took " + str(end - start))


def check_interface_locked(scene):
    if not scene.render.use_lock_interface and get_simulation_objects():
        scene.render.use_lock_interface = True
        print(
            "Squishy Volumes: Locked interface for rendering. See also https://docs.blender.org/api/master/bpy.app.handlers.html#note-on-altering-data"
        )


def register_handler():
    if check_interface_locked not in bpy.app.handlers.render_pre:
        bpy.app.handlers.render_pre.append(check_interface_locked)  # ty:ignore[invalid-argument-type]
        if get_print_debug_info():
            print("Squishy Volumes render pre check registered.")

    if frame_change_handler not in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.append(frame_change_handler)  # ty:ignore[invalid-argument-type]
        if get_print_debug_info():
            print("Squishy Volumes frame change handler registered.")


def unregister_handler():
    if frame_change_handler in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.remove(frame_change_handler)
        if get_print_debug_info():
            print("Squishy Volumes frame change handler unregistered.")

    if check_interface_locked in bpy.app.handlers.render_pre:
        bpy.app.handlers.render_pre.remove(check_interface_locked)
        if get_print_debug_info():
            print("Squishy Volumes render pre check unregistered.")
