# SPDX-License-Identifier: AGPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import bpy  # ty: ignore[unresolved-import]

from .bridge import SimulationHandle
from .capture_markers import capture_marker_frames
from .get_preferences import get_enable_timeline_markers, get_print_debug_info
from .popup import with_popup
from .squishy_volumes_properties import get_simulation_objects
from .util import force_ui_redraw

PROGRESS_INTERVAL = 0.25

_last_seen_capture_markers: dict[str, tuple[int, ...]] = {}


def capture_markers_changed(sim_obj: bpy.types.Object) -> bool:
    if not get_enable_timeline_markers() or bpy.context.scene is None:
        return False
    uuid = sim_obj.squishy_volumes.uuid
    frames = capture_marker_frames(bpy.context.scene, sim_obj)
    changed = _last_seen_capture_markers.get(uuid) != frames
    _last_seen_capture_markers[uuid] = frames
    return changed


def simulation_progressed(sim_obj: bpy.types.Object) -> bool:
    uuid = sim_obj.squishy_volumes.uuid
    sim_handle = SimulationHandle.get(uuid=uuid)
    if sim_handle is None:
        return False
    return bool(with_popup(uuid=uuid, f=sim_handle.poll))


def update_progress():
    should_redraw = False
    for sim_obj in get_simulation_objects():
        if capture_markers_changed(sim_obj):
            should_redraw = True
        if simulation_progressed(sim_obj):
            should_redraw = True

    if should_redraw:
        force_ui_redraw()

    return PROGRESS_INTERVAL


def is_updating():
    return bpy.app.timers.is_registered(update_progress)


def register_progress_update(*_scene):
    if not bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.register(update_progress, first_interval=PROGRESS_INTERVAL)
        if get_print_debug_info():
            print("Squishy Volumes progress update registered.")


def unregister_progress_update(*_scene):
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
