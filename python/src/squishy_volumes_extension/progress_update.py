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

import bpy  # ty: ignore[unresolved-import]

from .bridge import SimulationHandle
from .frame_change import sync_simulation
from .get_preferences import get_print_debug_info
from .popup import with_popup
from .squishy_volumes_properties import frame_to_load, get_simulation_objects
from .util import add_or_update_marker, force_ui_redraw, remove_marker

PROGRESS_INTERVAL = 0.25


def update_progress():
    should_redraw = False
    for sim_obj in get_simulation_objects():
        sim_props = sim_obj.squishy_volumes
        if not sim_props.sync:
            continue

        cleanup_capture_markers(sim_obj)
        add_or_update_marker(
            f"{sim_obj.name} Capture Start",
            sim_props.capture_start_frame,
        )
        add_or_update_marker(
            f"{sim_obj.name} Capture End",
            sim_props.capture_start_frame + sim_props.capture_frames - 1,
        )

        sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
        if sim_handle is None:
            continue

        def poll_and_true():
            return sim_handle.poll()  # noqa: B023

        if not with_popup(uuid=sim_props.uuid, f=poll_and_true):
            continue

        should_redraw = True

        cleanup_bake_markers(sim_obj)

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

        if sim_handle.loaded_frame != frame_to_load(
            sim_props,
            bpy.context.scene.frame_current,
        ):
            sync_simulation(sim_props, sim_handle, bpy.context.scene.frame_current)

    if should_redraw:
        force_ui_redraw()

    return PROGRESS_INTERVAL


def cleanup_capture_markers(sim_obj: bpy.types.Object):
    remove_marker(f"{sim_obj.name} Capture Start")
    remove_marker(f"{sim_obj.name} Capture End")


def cleanup_bake_markers(sim_obj: bpy.types.Object):
    remove_marker(f"{sim_obj.name} Bake Start")
    remove_marker(f"{sim_obj.name} Bake Latest")
    remove_marker(f"{sim_obj.name} Bake End")
    remove_marker(f"{sim_obj.name} Bake Latest & End")


def cleanup_markers(sim_obj: bpy.types.Object):
    cleanup_capture_markers(sim_obj)
    cleanup_bake_markers(sim_obj)


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
        bpy.app.handlers.render_init.append(unregister_progress_update)
    if register_progress_update not in bpy.app.handlers.render_complete:
        bpy.app.handlers.render_complete.append(register_progress_update)
    if register_progress_update not in bpy.app.handlers.render_cancel:
        bpy.app.handlers.render_cancel.append(register_progress_update)
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
