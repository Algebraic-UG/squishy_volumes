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
import time
import bpy

from .util import frame_to_load, get_simulation_by_uuid
from .nodes.drivers import remove_drivers
from .popup import with_popup
from .properties.util import get_output_objects
from .output import (
    sync_output,
)

from .bridge import (
    InputNames,
    context_exists,
    fetch_flat_attribute,
)


def sync(scene):
    for simulation in scene.blended_mpm_scene.simulations.values():
        if context_exists(simulation):
            sync_simulation(simulation, scene.frame_current)


# this is needed since the scene data block is sometimes (?)
# write protected in the frame handler.
def deferred_update(uuid, frame):
    simulation = get_simulation_by_uuid(uuid)

    if frame is None:
        simulation.loaded_frame = -1
    else:
        simulation.loaded_frame = frame

        def ffa(attribute):
            return fetch_flat_attribute(
                simulation,
                frame,
                json.dumps({"Setting": attribute}),
            )

        simulation.from_cache.grid_node_size = ffa("GridNodeSize")[0]
        simulation.from_cache.particle_size = ffa("ParticleSize")[0]
        simulation.from_cache.frames_per_second = int(ffa("FramesPerSecond")[0])
        simulation.from_cache.gravity = ffa("Gravity")

    return None  # unregister


def sync_simulation(simulation, frame):
    frame = frame_to_load(simulation, frame)
    bpy.app.timers.register(
        lambda: deferred_update(simulation.uuid, frame),
        first_interval=0.0,
    )

    if frame is None:
        return

    input_names = InputNames(simulation, frame)
    num_colliders = len(input_names.collider_names)

    desynced_objs = []
    for obj in get_output_objects(simulation):
        try:
            sync_output(simulation, obj, num_colliders, frame)
        except RuntimeError as e:
            desynced_objs.append((obj, e))
    if desynced_objs:
        for obj, _ in desynced_objs:
            obj.blended_mpm_object.simulation_uuid = ""
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

        with_popup(simulation, raise_)


def frame_change_handler(scene):
    start = time.time()
    sync(scene)
    end = time.time()
    print("Blended MPM: sync took " + str(end - start))


def check_interface_locked(scene):
    if not scene.render.use_lock_interface:
        scene.render.use_lock_interface = True
        print(
            "Blended MPM: Locked interface for rendering. See also https://docs.blender.org/api/master/bpy.app.handlers.html#note-on-altering-data"
        )


def register_frame_handler():
    if check_interface_locked not in bpy.app.handlers.render_pre:
        bpy.app.handlers.render_pre.append(check_interface_locked)
        print("Blended MPM render pre check registered.")

    if frame_change_handler not in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.append(frame_change_handler)
        print("Blended MPM frame change handler registered.")


def unregister_frame_handler():
    if frame_change_handler in bpy.app.handlers.frame_change_pre:
        bpy.app.handlers.frame_change_pre.remove(frame_change_handler)
        print("Blended MPM frame change handler unregistered.")

    if check_interface_locked in bpy.app.handlers.render_pre:
        bpy.app.handlers.render_pre.remove(check_interface_locked)
        print("Blended MPM render pre check unregistered.")
