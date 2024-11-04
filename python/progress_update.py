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

from .popup import with_popup
from .frame_change import sync_simulation
from .bridge import available_frames, context_exists, poll
from .util import add_or_update_marker, force_ui_redraw, remove_marker


PROGRESS_INTERVAL = 0.25


def update_progress():
    for simulation in bpy.context.scene.blended_mpm_scene.simulations.values():
        cleanup_markers(simulation)

        add_or_update_marker(
            f"{simulation.name} Capture Start", simulation.capture_start_frame
        )
        add_or_update_marker(
            f"{simulation.name} Capture End",
            simulation.capture_start_frame + simulation.capture_frames - 1,
        )

        if not context_exists(simulation):
            continue

        progress_json_string = with_popup(simulation, lambda: poll(simulation))
        if progress_json_string is None:
            continue
        simulation.progress_json_string = progress_json_string

        computed_frames = available_frames(simulation)
        if not computed_frames:
            continue

        add_or_update_marker(
            f"{simulation.name} Bake Start",
            simulation.display_start_frame,
        )

        latest_frame = simulation.display_start_frame + computed_frames - 1
        end_frame = simulation.display_start_frame + simulation.bake_frames - 1
        if latest_frame != end_frame:
            add_or_update_marker(f"{simulation.name} Bake Latest", latest_frame)
            add_or_update_marker(f"{simulation.name} Bake End", end_frame)
        else:
            add_or_update_marker(f"{simulation.name} Bake Latest & End", end_frame)

        should_show = bpy.context.scene.frame_current - simulation.display_start_frame
        if should_show != simulation.loaded_frame and should_show < computed_frames:
            sync_simulation(simulation, bpy.context.scene.frame_current)

    # TODO: check if anything has changed before calling this
    force_ui_redraw()

    return PROGRESS_INTERVAL


def cleanup_markers(simulation):
    remove_marker(f"{simulation.name} Capture Start")
    remove_marker(f"{simulation.name} Capture End")
    remove_marker(f"{simulation.name} Bake Start")
    remove_marker(f"{simulation.name} Bake Latest")
    remove_marker(f"{simulation.name} Bake End")
    remove_marker(f"{simulation.name} Bake Latest & End")


def register_progress_update():
    if not bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.register(update_progress, first_interval=PROGRESS_INTERVAL)
        print("Blended MPM progress update registered.")


def unregister_progress_update():
    for simulation in bpy.context.scene.blended_mpm_scene.simulations.values():
        cleanup_markers(simulation)

    if bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.unregister(update_progress)
        print("Blended MPM progress update unregistered.")
