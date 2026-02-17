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

import json
import numpy
from typing import Any, Self

from .shim import *
from .hint_at_info import *


@hint_at_info
def build_info() -> dict[str, Any]:
    return json.loads(squishy_volumes_wrap.build_info_as_json())


@hint_at_info
def test(spacing, layers):
    vs = bpy.context.active_object.data.vertices
    ts = bpy.context.active_object.data.loop_triangles

    array = numpy.array(
        object=[
            spacing,
            layers,
            vs[0].co.x,
            vs[0].co.y,
            vs[0].co.z,
            vs[1].co.x,
            vs[1].co.y,
            vs[1].co.z,
            vs[2].co.x,
            vs[2].co.y,
            vs[2].co.z,
            ts[0].normal.x,
            ts[0].normal.y,
            ts[0].normal.z,
        ],
        dtype="float32",
    )
    positions = squishy_volumes_wrap.test(array)

    obj = bpy.data.objects.get("test")
    if obj is None:
        obj = bpy.data.objects.new("test", bpy.data.meshes.new("test"))
        bpy.context.collection.objects.link(obj)

    num_floats = positions.size
    num_vertices = num_floats // 3

    obj.data.clear_geometry()
    obj.data.vertices.add(num_vertices)  # Pre-allocate vertex space
    obj.data.vertices.foreach_set("co", positions)  # Set all coordinates in one go


class SimulationInput:
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
        return SimulationInput(
            handle=squishy_volumes_wrap.SimulationInput.new(
                uuid,
                directory,
                json.dumps(input_header),
                max_bytes_on_disk,
            )
        )  # ty:ignore[invalid-return-type]

    @hint_at_info
    def start_frame(self, *, frame_start: dict[str, Any]):
        self.handle.start_frame(json.dumps(frame_start))

    @hint_at_info
    def record_input_float(self, *, meta: dict[str, Any], bulk: numpy.ndarray):
        self.handle.record_input_float(json.dumps(meta), bulk)

    @hint_at_info
    def record_input_int(self, *, meta: dict[str, Any], bulk: numpy.ndarray):
        self.handle.record_input_int(json.dumps(meta), bulk)

    @hint_at_info
    def finish_frame(self):
        self.handle.finish_frame()

    @hint_at_info
    def drop(self):
        self.handle.drop()


_simulations: dict[str, "Simulation"] = {}


class Simulation:
    def __init__(self, *, handle: squishy_volumes_wrap.Simulation):
        _simulations[handle.uuid()] = self
        self.handle = handle
        self.last_error = ""
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
        return Simulation(handle=squishy_volumes_wrap.Simulation.new())  # ty:ignore[invalid-return-type]

    @hint_at_info
    @staticmethod
    def load(*, uuid: str, directory: str) -> Self:
        return Simulation(handle=squishy_volumes_wrap.Simulation.load(uuid, directory))  # ty:ignore[invalid-return-type]

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
        time_step: float,
        explicit: bool,
        debug_mode: bool,
        adaptive_time_steps: bool,
        next_frame: int,
        number_of_frames: int,
        max_bytes_on_disk: int,
    ):
        self.handle.start_compute(
            time_step,
            explicit,
            debug_mode,
            adaptive_time_steps,
            next_frame,
            number_of_frames,
            max_bytes_on_disk,
        )

    @hint_at_info
    def pause_compute(self):
        self.handle.pause_compute()

    @hint_at_info
    def available_frames(self) -> int:
        return self.handle.available_frames()

    @hint_at_info
    def available_attributes(self, *, frame: int) -> list[dict[str, Any]]:
        return [json.loads(s) for s in self.handle.available_attributes(frame)]

    @hint_at_info
    def fetch_flat_attribute(
        self, *, frame: int, attribute: dict[str, Any]
    ) -> numpy.ndarray:
        return self.handle.fetch_flat_attribute(frame, json.dumps(attribute))

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
