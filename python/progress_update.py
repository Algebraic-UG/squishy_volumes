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

from .popup import with_popup
from .frame_change import sync_simulation
from .bridge import Simulation
from .util import add_or_update_marker, force_ui_redraw, remove_marker, frame_to_load


PROGRESS_INTERVAL = 0.25


def update_progress():
    should_redraw = False
    for simulation in bpy.context.scene.squishy_volumes_scene.simulations.values():  # ty:ignore[unresolved-attribute]
        cleanup_markers(simulation)

        add_or_update_marker(
            f"{simulation.name} Capture Start", simulation.capture_start_frame
        )
        add_or_update_marker(
            f"{simulation.name} Capture End",
            simulation.capture_start_frame + simulation.capture_frames - 1,
        )

        if not simulation.sync:
            continue

        sim = Simulation.get(uuid=simulation.uuid)
        if sim is None:
            continue

        progess = sim.progress

        def poll_and_true():
            sim.poll()
            return True

        if not with_popup(uuid=simulation.uuid, f=poll_and_true):
            sim.drop()
            continue

        if progess != sim.progress:
            should_redraw = True

        add_or_update_marker(
            f"{simulation.name} Bake Start",
            simulation.display_start_frame,
        )

        computed_frames = sim.available_frames()
        if computed_frames == 0:
            continue

        latest_frame = simulation.display_start_frame + computed_frames - 1
        end_frame = simulation.display_start_frame + simulation.bake_frames - 1
        if latest_frame != end_frame:
            add_or_update_marker(f"{simulation.name} Bake Latest", latest_frame)
            add_or_update_marker(f"{simulation.name} Bake End", end_frame)
        else:
            add_or_update_marker(f"{simulation.name} Bake Latest & End", end_frame)

        if simulation.loaded_frame != frame_to_load(
            simulation,
            bpy.context.scene.frame_current,  # ty:ignore[possibly-missing-attribute]
        ):
            sync_simulation(sim, simulation, bpy.context.scene.frame_current)  # ty:ignore[possibly-missing-attribute]

    if should_redraw:
        force_ui_redraw()

    return PROGRESS_INTERVAL


def cleanup_markers(simulation):
    remove_marker(f"{simulation.name} Capture Start")
    remove_marker(f"{simulation.name} Capture End")
    remove_marker(f"{simulation.name} Bake Start")
    remove_marker(f"{simulation.name} Bake Latest")
    remove_marker(f"{simulation.name} Bake End")
    remove_marker(f"{simulation.name} Bake Latest & End")


def is_updating():
    return bpy.app.timers.is_registered(update_progress)


def register_progress_update(*_scene):
    if not bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.register(update_progress, first_interval=PROGRESS_INTERVAL)
        print("Squishy Volumes progress update registered.")


def unregister_progress_update(*_scene):
    for simulation in bpy.context.scene.squishy_volumes_scene.simulations.values():  # ty:ignore[unresolved-attribute]
        cleanup_markers(simulation)

    if bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.unregister(update_progress)
        print("Squishy Volumes progress update unregistered.")


def register_progress_update_toggle():
    if unregister_progress_update not in bpy.app.handlers.render_init:
        bpy.app.handlers.render_init.append(unregister_progress_update)  # ty:ignore[invalid-argument-type]
    if register_progress_update not in bpy.app.handlers.render_complete:
        bpy.app.handlers.render_complete.append(register_progress_update)  # ty:ignore[invalid-argument-type]
    if register_progress_update not in bpy.app.handlers.render_cancel:
        bpy.app.handlers.render_cancel.append(register_progress_update)  # ty:ignore[invalid-argument-type]
    print("Squishy Volumes progress update toggle on render registered.")


def unregister_progress_update_toggle():
    if unregister_progress_update in bpy.app.handlers.render_init:
        bpy.app.handlers.render_init.remove(unregister_progress_update)
    if register_progress_update in bpy.app.handlers.render_complete:
        bpy.app.handlers.render_complete.remove(register_progress_update)
    if register_progress_update in bpy.app.handlers.render_cancel:
        bpy.app.handlers.render_cancel.remove(register_progress_update)
    print("Squishy Volumes progress update toggle on render unregistered.")
