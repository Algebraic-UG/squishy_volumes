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

import bpy
from bpy.app.handlers import persistent

from .bridge import SimulationHandle
from .capture_markers import capture_marker_frames
from .get_preferences import debug_log, get_enable_timeline_markers
from .popup import show_message
from .registration import add_handler, remove_handler
from .squishy_volumes_properties import get_simulation_objects
from .util import force_ui_redraw

PROGRESS_INTERVAL = 0.25

_last_seen_capture_markers: dict[str, tuple[int, ...]] = {}
_shown_bake_errors: dict[str, tuple[list[str], int]] = {}


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
        _shown_bake_errors.pop(uuid, None)
        return False
    progressed = sim_handle.poll()
    show_new_bake_errors(uuid, sim_handle)
    return progressed


def unseen_bake_errors(
    errors: list[str], shown: tuple[list[str], int] | None
) -> list[str]:
    if shown is None or shown[0] is not errors:
        return list(errors)
    return errors[shown[1] :]


def show_new_bake_errors(uuid: str, sim_handle: SimulationHandle):
    errors = sim_handle.bake_errors
    unseen = unseen_bake_errors(errors, _shown_bake_errors.get(uuid))
    _shown_bake_errors[uuid] = (errors, len(errors))
    if unseen:
        show_message(uuid, "\n\n".join(unseen))


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


@persistent
def register_progress_update(*_scene):
    if not bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.register(
            update_progress, first_interval=PROGRESS_INTERVAL, persistent=True
        )
        debug_log("progress update registered.")


@persistent
def unregister_progress_update(*_scene):
    if bpy.app.timers.is_registered(update_progress):
        bpy.app.timers.unregister(update_progress)
        debug_log("progress update unregistered.")


def render_toggle_handlers():
    return [
        (bpy.app.handlers.render_init, unregister_progress_update),
        (bpy.app.handlers.render_complete, register_progress_update),
        (bpy.app.handlers.render_cancel, register_progress_update),
    ]


def register_progress_update_toggle():
    for handlers, handler in render_toggle_handlers():
        add_handler(handlers, handler)
    debug_log("progress update toggle on render registered.")


def unregister_progress_update_toggle():
    for handlers, handler in render_toggle_handlers():
        remove_handler(handlers, handler)
    debug_log("progress update toggle on render unregistered.")
