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

from .nodes.drivers import remove_drivers
from .popup import with_popup
from .output import (
    sync_output,
)

from .bridge import Simulation
from .util import frame_to_load
from .properties.squishy_volumes_object import get_output_objects
from .properties.squishy_volumes_simulation import Squishy_Volumes_Simulation


def sync(scene):
    for simulation in scene.squishy_volumes_scene.simulations.values():
        if not simulation.sync:
            continue
        sim = Simulation.get(uuid=simulation.uuid)
        if sim is None:
            continue
        sync_simulation(sim, simulation, scene.frame_current)


def sync_simulation(
    sim: Simulation,
    simulation: Squishy_Volumes_Simulation,
    frame: int,
):
    frame = frame_to_load(simulation, frame)

    if frame is None:
        return

    simulation.has_loaded_frame = True
    simulation.loaded_frame = frame

    # TODO
    num_colliders = 0

    desynced_objs = []
    for obj in get_output_objects(simulation):
        try:
            sync_output(sim, obj, num_colliders, frame)
        except RuntimeError as e:
            desynced_objs.append((obj, e))

    if desynced_objs:
        for obj, _ in desynced_objs:
            obj.squishy_volumes_object.simulation_uuid = ""
            remove_drivers(obj)

        def raise_():
            message = """These output objects could not be synced and
have been decoupled from the output of the simulation.
(Most likely, the respective input object
is now incompatible or gone.)

"""
            for obj, e in desynced_objs:
                message += f"{obj.name}: {str(e)}"

            raise RuntimeError(message)

        with_popup(uuid=simulation.uuid, f=raise_)


def frame_change_handler(scene):
    start = time.time()
    sync(scene)
    end = time.time()
    print("Squishy Volumes: sync took " + str(end - start))


def check_interface_locked(scene):
    if not scene.render.use_lock_interface:
        scene.render.use_lock_interface = True
        print(
            "Squishy Volumes: Locked interface for rendering. See also https://docs.blender.org/api/master/bpy.app.handlers.html#note-on-altering-data"
        )


def register_handler():
    if check_interface_locked not in bpy.app.handlers.render_pre:
        bpy.app.handlers.render_pre.append(check_interface_locked)  # ty:ignore[invalid-argument-type]
        print("Squishy Volumes render pre check registered.")

    if frame_change_handler not in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.append(frame_change_handler)  # ty:ignore[invalid-argument-type]
        print("Squishy Volumes frame change handler registered.")


def unregister_handler():
    if frame_change_handler in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.remove(frame_change_handler)
        print("Squishy Volumes frame change handler unregistered.")

    if check_interface_locked in bpy.app.handlers.render_pre:
        bpy.app.handlers.render_pre.remove(check_interface_locked)
        print("Squishy Volumes render pre check unregistered.")
