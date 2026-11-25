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

from collections.abc import Generator

import bpy

from ..bridge import SimulationHandle, SimulationInputHandle
from ..capture_markers import (
    capture_markers_error,
    range_to_record,
    store_capture_range,
)
from ..capture_markers import set_capture_range as set_stored_and_marked_range
from ..errors import CaptureRangeError
from ..input_capture import capture_input_frame, create_input_header
from ..properties import stored_range
from ..util import giga_f32_to_u64
from .simulation import reload, scene_or_current, update_frame_driver


def capture_range(
    sim_obj: bpy.types.Object, *, scene: bpy.types.Scene | None = None
) -> tuple[int, int]:
    scene = scene_or_current(scene)
    frames = range_to_record(scene, sim_obj)
    if frames is None:
        raise CaptureRangeError(capture_markers_error(scene, sim_obj))
    return frames


def set_capture_range(
    sim_obj: bpy.types.Object,
    start: int,
    end: int,
    *,
    scene: bpy.types.Scene | None = None,
):
    if end < start:
        raise CaptureRangeError(f"The capture end {end} is before its start {start}")
    set_stored_and_marked_range(scene_or_current(scene), sim_obj, start, end)


def record_input(
    sim_obj: bpy.types.Object, *, scene: bpy.types.Scene | None = None
) -> int:
    recorded_frames = 0
    for recorded_frames in iter_record_input(sim_obj, scene=scene):
        pass
    return recorded_frames


def iter_record_input(
    sim_obj: bpy.types.Object, *, scene: bpy.types.Scene | None = None
) -> Generator[int]:
    scene = scene_or_current(scene)
    sim_props = sim_obj.squishy_volumes
    start, end = capture_range(sim_obj, scene=scene)
    previous_range = stored_range(sim_props)
    store_capture_range(sim_props, start, end)

    sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
    if sim_handle is not None:
        sim_handle.drop()

    try:
        yield from _record_input(scene, sim_props, start, end)
        SimulationHandle.new()
    except BaseException:
        store_capture_range(sim_props, *previous_range)
        if sim_handle is not None:
            reload(sim_obj)
        raise

    update_frame_driver(sim_props, start)
    release_cache_file(sim_props)


def _record_input(
    scene: bpy.types.Scene, sim_props, start: int, end: int
) -> Generator[int]:
    prior_frame = scene.frame_current
    try:
        scene.frame_set(start)
        input_handle = SimulationInputHandle.new(
            uuid=sim_props.uuid,
            directory=sim_props.directory,
            input_header=create_input_header(sim_props),
            max_bytes_on_disk=giga_f32_to_u64(sim_props.max_giga_bytes_on_disk),
        )
        yield from _record_frames(scene, sim_props, input_handle, range(start, end + 1))
    finally:
        scene.frame_set(prior_frame)


def release_cache_file(sim_props):
    cache_file = sim_props.cache_file
    if cache_file is None:
        return
    # see https://github.com/Algebraic-UG/squishy_volumes/issues/421
    layer = cache_file.layers.new(filepath="//squishy_volumes_release_handle.abc")
    cache_file.layers.remove(layer)


def _record_frames(
    scene: bpy.types.Scene,
    sim_props,
    input_handle: SimulationInputHandle,
    frames: range,
) -> Generator[int]:
    try:
        for recorded_frames, frame in enumerate(frames, start=1):
            scene.frame_set(frame)
            capture_input_frame(sim_props=sim_props, sim_input_handle=input_handle)
            yield recorded_frames
    except BaseException:
        input_handle.drop()
        raise
