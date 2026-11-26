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

import time

import bpy

from ..bridge import SimulationHandle
from ..errors import (
    BakeFailed,
    BakeTimeout,
    InvalidBakeRange,
    SimulationNotLoaded,
)
from ..get_preferences import get_sanity_check_allowed_disk_space
from ..util import giga_f32_to_u64
from .record import release_cache_file


def is_baking(sim_obj: bpy.types.Object) -> bool:
    sim_handle = SimulationHandle.get(uuid=sim_obj.squishy_volumes.uuid)
    return sim_handle is not None and sim_handle.baking()


def baked_frames(sim_obj: bpy.types.Object) -> int:
    sim_handle = SimulationHandle.get(uuid=sim_obj.squishy_volumes.uuid)
    return 0 if sim_handle is None else sim_handle.available_frames()


def bake(
    sim_obj: bpy.types.Object,
    *,
    from_frame: int | None = None,
    bake_frames: int | None = None,
):
    sim_props = sim_obj.squishy_volumes
    sim_handle = _loaded_handle(sim_obj)
    available_frames = sim_handle.available_frames()
    if from_frame is None:
        next_frame = available_frames
    elif 0 <= from_frame < available_frames:
        next_frame = from_frame + 1
    else:
        raise InvalidBakeRange(
            f"Can't bake from frame {from_frame}, "
            f"{sim_obj.name} has frames 0 to {available_frames - 1}"
        )
    number_of_frames = sim_props.bake_frames if bake_frames is None else bake_frames
    if next_frame >= number_of_frames:
        return

    sim_handle.last_error = None
    release_cache_file(sim_props)
    sim_handle.start_compute(
        compute_settings={
            "time_step": sim_props.time_step,
            "gpu": None
            if sim_props.compute_device == "CPU"
            else sim_props.compute_device,
            "adaptive_time_steps": sim_props.adaptive_time_steps,
            "next_frame": next_frame,
            "number_of_frames": number_of_frames,
            "max_bytes_on_disk": giga_f32_to_u64(sim_props.max_giga_bytes_on_disk),
            "check_disk_space": get_sanity_check_allowed_disk_space(),
        }
    )


def pause_bake(sim_obj: bpy.types.Object):
    _loaded_handle(sim_obj).pause_compute()


def bake_errors(sim_obj: bpy.types.Object) -> list[str]:
    sim_handle = SimulationHandle.get(uuid=sim_obj.squishy_volumes.uuid)
    if sim_handle is None:
        return []
    sim_handle.poll()
    return list(sim_handle.bake_errors)


def clear_bake_errors(sim_obj: bpy.types.Object):
    _loaded_handle(sim_obj).clear_errors()


def wait_until_baked(
    sim_obj: bpy.types.Object, *, timeout_sec: float, poll_interval_sec: float = 0.01
) -> int:
    sim_handle = _loaded_handle(sim_obj)
    deadline = time.monotonic() + timeout_sec
    while True:
        baking = sim_handle.baking()
        sim_handle.poll()
        if not baking:
            if sim_handle.bake_errors:
                raise BakeFailed(
                    f"Baking {sim_obj.name} failed:\n"
                    + "\n\n".join(sim_handle.bake_errors)
                )
            return sim_handle.available_frames()
        if time.monotonic() > deadline:
            raise BakeTimeout(
                f"{sim_obj.name} is still baking after {timeout_sec} seconds"
            )
        time.sleep(poll_interval_sec)


def _loaded_handle(sim_obj: bpy.types.Object) -> SimulationHandle:
    sim_handle = SimulationHandle.get(uuid=sim_obj.squishy_volumes.uuid)
    if sim_handle is None:
        raise SimulationNotLoaded(
            f"{sim_obj.name} has no loaded cache, record its input or reload it first"
        )
    return sim_handle
