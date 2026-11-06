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

from .get_preferences import get_enable_timeline_markers
from .squishy_volumes_properties import (
    Squishy_Volumes_Properties_Simulation,
    stored_range,
)


def capture_markers(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
) -> list[bpy.types.TimelineMarker]:
    matching = [m for m in scene.timeline_markers if m.name == sim_obj.name]
    return sorted(matching, key=lambda marker: marker.frame)


def capture_marker_frames(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
) -> tuple[int, ...]:
    return tuple(marker.frame for marker in capture_markers(scene, sim_obj))


def capture_range(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
) -> tuple[int, int] | None:
    markers = capture_markers(scene, sim_obj)
    if len(markers) != 2:
        return None
    return markers[0].frame, markers[1].frame


def capture_markers_error(scene: bpy.types.Scene, sim_obj: bpy.types.Object) -> str:
    found = len(capture_markers(scene, sim_obj))
    return f"Expected 2 timeline markers named '{sim_obj.name}', found {found}"


def store_capture_range(
    sim_props: Squishy_Volumes_Properties_Simulation, start: int, end: int
):
    sim_props.capture_start_frame = start
    sim_props.capture_end_frame = end


def range_to_record(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
) -> tuple[int, int] | None:
    if get_enable_timeline_markers():
        return capture_range(scene, sim_obj)
    return stored_range(sim_obj.squishy_volumes)


def place_capture_markers(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object, start: int, end: int
):
    remove_capture_markers(scene, sim_obj)
    scene.timeline_markers.new(sim_obj.name, frame=start)
    scene.timeline_markers.new(sim_obj.name, frame=end)


def set_capture_range(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object, start: int, end: int
):
    store_capture_range(sim_obj.squishy_volumes, start, end)
    if get_enable_timeline_markers():
        place_capture_markers(scene, sim_obj, start, end)


def remove_capture_markers(scene: bpy.types.Scene, sim_obj: bpy.types.Object):
    for marker in capture_markers(scene, sim_obj):
        scene.timeline_markers.remove(marker)


def remove_capture_markers_if_enabled(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
):
    if get_enable_timeline_markers():
        remove_capture_markers(scene, sim_obj)


def superfluous_capture_markers(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
) -> list[bpy.types.TimelineMarker]:
    return capture_markers(scene, sim_obj)[1:-1]


def remove_superfluous_capture_markers(
    scene: bpy.types.Scene, sim_obj: bpy.types.Object
):
    for marker in superfluous_capture_markers(scene, sim_obj):
        scene.timeline_markers.remove(marker)
