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

import platform
import bpy
from bpy.app.handlers import persistent


import json
import numpy
from typing import Any, Self

from .shim import *
from .get_preferences import get_print_debug_info
from .hint_at_info import *


@hint_at_info
def build_info() -> dict[str, Any]:
    return json.loads(squishy_volumes_wrap.build_info_as_json())


@hint_at_info
def available_gpus() -> list[str]:
    return squishy_volumes_wrap.available_gpus()


DETECTED_DEVICES = [("CPU", f"CPU ({platform.processor()})", "")] + [
    (gpu, gpu, "") for gpu in available_gpus()
]


class SimulationInputHandle:
    def __init__(self, *, handle: squishy_volumes_wrap.SimulationInput):
        self.handle = handle

    @hint_at_info
    @staticmethod
    def new(
        *,
        uuid: str,
        directory: str,
        input_header: dict[str, Any],
        max_bytes_on_disk: int,
    ) -> Self:
        return SimulationInputHandle(
            handle=squishy_volumes_wrap.SimulationInput.new(
                uuid=uuid,
                directory=directory,
                input_header=json.dumps(input_header),
                max_bytes_on_disk=max_bytes_on_disk,
            )
        )  # ty:ignore[invalid-return-type]

    @hint_at_info
    def start_frame(self, *, frame_start: dict[str, Any]):
        self.handle.start_frame(frame_start=json.dumps(frame_start))

    @hint_at_info
    def record_input_bool(self, *, meta: dict[str, Any], bulk: numpy.ndarray):
        self.handle.record_input_bool(meta=json.dumps(meta), bulk=bulk)

    @hint_at_info
    def record_input_float(self, *, meta: dict[str, Any], bulk: numpy.ndarray):
        self.handle.record_input_float(meta=json.dumps(meta), bulk=bulk)

    @hint_at_info
    def record_input_int(self, *, meta: dict[str, Any], bulk: numpy.ndarray):
        self.handle.record_input_int(meta=json.dumps(meta), bulk=bulk)

    @hint_at_info
    def finish_frame(self):
        self.handle.finish_frame()

    @hint_at_info
    def drop(self):
        self.handle.drop()


_simulations: dict[str, "SimulationHandle"] = {}


class SimulationHandle:
    def __init__(self, *, handle: squishy_volumes_wrap.Simulation):
        _simulations[handle.uuid()] = self
        self.handle = handle
        self.last_error = None
        self.progress = None

    @staticmethod
    def exists(*, uuid: str) -> bool:
        return uuid in _simulations

    @staticmethod
    def get(*, uuid: str) -> None | Self:
        return _simulations.get(uuid)  # ty:ignore[invalid-return-type]

    @hint_at_info
    @staticmethod
    def new() -> Self:
        return SimulationHandle(handle=squishy_volumes_wrap.Simulation.new())  # ty:ignore[invalid-return-type]

    @hint_at_info
    @staticmethod
    def load(*, uuid: str, directory: str) -> Self:
        return SimulationHandle(
            handle=squishy_volumes_wrap.Simulation.load(
                uuid=uuid,
                directory=directory,
            )
        )  # ty:ignore[invalid-return-type]

    @hint_at_info
    def input_header(self) -> dict[str, Any]:
        return json.loads(self.handle.input_header())

    @hint_at_info
    def poll(self):
        progress = self.handle.poll()
        if progress is None:
            self.progress = None
        else:
            self.progress = json.loads(progress)

    @hint_at_info
    def computing(self) -> bool:
        return self.handle.computing()

    @hint_at_info
    def start_compute(
        self,
        *,
        compute_settings: dict[str, Any],
    ):
        self.handle.start_compute(
            compute_settings=json.dumps(compute_settings),
        )

    @hint_at_info
    def pause_compute(self):
        self.handle.pause_compute()

    @hint_at_info
    def available_frames(self) -> int:
        return self.handle.available_frames()

    @hint_at_info
    def available_attributes(self) -> list[dict[str, Any]]:
        return [json.loads(s) for s in self.handle.available_attributes()]

    @hint_at_info
    def fetch_flat_attribute_f32(
        self, *, frame: int, attribute: dict[str, Any]
    ) -> numpy.ndarray:
        return self.handle.fetch_flat_attribute_f32(
            frame=frame,
            attribute=json.dumps(attribute),
        )

    @hint_at_info
    def fetch_flat_attribute_i32(
        self, *, frame: int, attribute: dict[str, Any]
    ) -> numpy.ndarray:
        return self.handle.fetch_flat_attribute_i32(
            frame=frame,
            attribute=json.dumps(attribute),
        )

    @hint_at_info
    def stats(self) -> dict[str, Any]:
        return json.loads(self.handle.stats())

    @hint_at_info
    def drop(self):
        _simulations.pop(self.handle.uuid())
        self.handle.drop()

    @hint_at_info
    @staticmethod
    def drop_all():
        for simulation in _simulations.values():
            simulation.handle.drop()


# simulation objects can be deleted through uncontrolled means, for example, undo
@persistent
def prune_simulation_handles(scene):
    live_uuids = [
        obj.squishy_volumes.uuid
        for obj in bpy.data.objects
        if obj.squishy_volumes.type == "Simulation"
    ]
    to_drop = [uuid for uuid in _simulations.keys() if uuid not in live_uuids]
    for uuid in to_drop:
        _simulations[uuid].drop()


def register_prune_simulation_handles():
    if prune_simulation_handles not in bpy.app.handlers.depsgraph_update_post:
        bpy.app.handlers.depsgraph_update_post.append(prune_simulation_handles)
    if get_print_debug_info():
        print("Squishy Volumes prune simulation handles registered.")


def unregister_prune_simulation_handles():
    if prune_simulation_handles in bpy.app.handlers.depsgraph_update_post:
        bpy.app.handlers.depsgraph_update_post.remove(prune_simulation_handles)
    if get_print_debug_info():
        print("Squishy Volumes prune simulation handles unregistered.")
