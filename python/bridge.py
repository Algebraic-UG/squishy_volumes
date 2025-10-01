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

import json

from .shim import *

squishy_volumes_context_dict = {}


def giga_f32_to_u64(giga_float):
    return int(float(giga_float) * 1e9)


def build_info():
    return json.loads(squishy_volumes_wrap.build_info_as_json())


def new_simulation(simulation, serialized_setup):
    drop_context(simulation)

    squishy_volumes_context_dict[simulation.uuid] = squishy_volumes_wrap.new(
        simulation.uuid,
        simulation.cache_directory,
        serialized_setup,
        giga_f32_to_u64(simulation.max_giga_bytes_on_disk),
    )

    return True


def load_simulation(simulation):
    drop_context(simulation)

    squishy_volumes_context_dict[simulation.uuid] = squishy_volumes_wrap.load(
        simulation.uuid,
        simulation.cache_directory,
        giga_f32_to_u64(simulation.max_giga_bytes_on_disk),
    )


def drop_context(simulation):
    if simulation.uuid in squishy_volumes_context_dict:
        squishy_volumes_context_dict.pop(simulation.uuid).drop()


def context_exists(simulation):
    return simulation.uuid in squishy_volumes_context_dict


def poll(simulation):
    return squishy_volumes_context_dict[simulation.uuid].poll()


def computing(simulation):
    return (
        context_exists(simulation)
        and squishy_volumes_context_dict[simulation.uuid].computing()
    )


def start_compute_initial_frame(simulation):
    squishy_volumes_context_dict[simulation.uuid].start_compute(
        simulation.time_step,
        simulation.explicit,
        simulation.debug_mode,
        0,
        1,
        giga_f32_to_u64(simulation.max_giga_bytes_on_disk),
    )


def start_compute(simulation, from_frame):
    squishy_volumes_context_dict[simulation.uuid].start_compute(
        simulation.time_step,
        simulation.explicit,
        simulation.debug_mode,
        from_frame,
        simulation.bake_frames,
        giga_f32_to_u64(simulation.max_giga_bytes_on_disk),
    )


def pause_compute(simulation):
    squishy_volumes_context_dict[simulation.uuid].pause_compute()


def available_frames(simulation):
    if not context_exists(simulation):
        return 0
    return squishy_volumes_context_dict[simulation.uuid].available_frames()


def available_attributes(simulation, frame):
    return squishy_volumes_context_dict[simulation.uuid].available_attributes(frame)


def fetch_flat_attribute(simulation, frame, attribute_json):
    return squishy_volumes_context_dict[simulation.uuid].fetch_flat_attribute(
        frame, attribute_json
    )


def cleanup_native():
    for context in squishy_volumes_context_dict.values():
        context.drop()
    squishy_volumes_context_dict.clear()


class InputNames:
    def __init__(self, simulation, frame):
        self.solid_names = set()
        self.fluid_names = set()
        self.collider_names = set()
        self.mesh_names = set()
        if not context_exists(simulation):
            return
        for attribute_json in available_attributes(simulation, frame):
            attribute = json.loads(attribute_json)
            if "Object" in attribute:
                name = attribute["Object"]["name"]
                object_attribute = attribute["Object"]["attribute"]
                if "Solid" in object_attribute:
                    self.solid_names.add(name)
                if "Fluid" in object_attribute:
                    self.fluid_names.add(name)
                if "Collider" in object_attribute:
                    self.collider_names.add(name)
            if "Mesh" in attribute:
                name = attribute["Mesh"]["name"]
                self.mesh_names.add(name)
