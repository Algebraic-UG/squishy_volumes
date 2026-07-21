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

from .preferences import get_print_debug_info
from .popup import with_popup
from .frame_change import sync_simulation
from .bridge import SimulationHandle
from .util import add_or_update_marker, force_ui_redraw, remove_marker
from .squishy_volumes_properties import frame_to_load, get_simulation_objects


PROGRESS_INTERVAL = 0.25


def update_progress():
    should_redraw = False
    for sim_obj in get_simulation_objects():
        cleanup_markers(sim_obj)

        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        add_or_update_marker(
            f"{sim_obj.name} Capture Start",
            sim_props.capture_start_frame,
        )
        add_or_update_marker(
            f"{sim_obj.name} Capture End",
            sim_props.capture_start_frame + sim_props.capture_frames - 1,
        )

        if not sim_props.sync:
            continue

        sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
        if sim_handle is None:
            continue

        if sim_handle.last_error is not None:
            continue

        progess = sim_handle.progress

        def poll_and_true():
            sim_handle.poll()
            return True

        if not with_popup(uuid=sim_props.uuid, f=poll_and_true):
            continue

        if progess != sim_handle.progress:
            should_redraw = True

        add_or_update_marker(
            f"{sim_obj.name} Bake Start",
            sim_props.display_start_frame,
        )

        computed_frames = sim_handle.available_frames()
        if computed_frames == 0:
            continue

        latest_frame = sim_props.display_start_frame + computed_frames - 1
        end_frame = sim_props.display_start_frame + sim_props.bake_frames - 1
        if latest_frame != end_frame:
            add_or_update_marker(f"{sim_obj.name} Bake Latest", latest_frame)
            add_or_update_marker(f"{sim_obj.name} Bake End", end_frame)
        else:
            add_or_update_marker(f"{sim_obj.name} Bake Latest & End", end_frame)

        if sim_props.loaded_frame != frame_to_load(
            sim_props,
            bpy.context.scene.frame_current,  # ty:ignore[possibly-missing-attribute]
        ):
            sync_simulation(sim_props, sim_handle, bpy.context.scene.frame_current)  # ty:ignore[possibly-missing-attribute]

    if should_redraw:
        force_ui_redraw()

    return PROGRESS_INTERVAL


def cleanup_markers(sim_obj: bpy.types.Object):
    remove_marker(f"{sim_obj.name} Capture Start")
    remove_marker(f"{sim_obj.name} Capture End")
    remove_marker(f"{sim_obj.name} Bake Start")
    remove_marker(f"{sim_obj.name} Bake Latest")
    remove_marker(f"{sim_obj.name} Bake End")
    remove_marker(f"{sim_obj.name} Bake Latest & End")


def is_updating():
    return bpy.app.timers.is_registered(update_progress)


def register_progress_update(*_scene):
    if not bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.register(update_progress, first_interval=PROGRESS_INTERVAL)
        if get_print_debug_info():
            print("Squishy Volumes progress update registered.")


def unregister_progress_update(*_scene):
    for sim_obj in get_simulation_objects():
        cleanup_markers(sim_obj)

    if bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.unregister(update_progress)
        if get_print_debug_info():
            print("Squishy Volumes progress update unregistered.")


def register_progress_update_toggle():
    if unregister_progress_update not in bpy.app.handlers.render_init:
        bpy.app.handlers.render_init.append(unregister_progress_update)  # ty:ignore[invalid-argument-type]
    if register_progress_update not in bpy.app.handlers.render_complete:
        bpy.app.handlers.render_complete.append(register_progress_update)  # ty:ignore[invalid-argument-type]
    if register_progress_update not in bpy.app.handlers.render_cancel:
        bpy.app.handlers.render_cancel.append(register_progress_update)  # ty:ignore[invalid-argument-type]
    if get_print_debug_info():
        print("Squishy Volumes progress update toggle on render registered.")


def unregister_progress_update_toggle():
    if unregister_progress_update in bpy.app.handlers.render_init:
        bpy.app.handlers.render_init.remove(unregister_progress_update)
    if register_progress_update in bpy.app.handlers.render_complete:
        bpy.app.handlers.render_complete.remove(register_progress_update)
    if register_progress_update in bpy.app.handlers.render_cancel:
        bpy.app.handlers.render_cancel.remove(register_progress_update)
    if get_print_debug_info():
        print("Squishy Volumes progress update toggle on render unregistered.")
