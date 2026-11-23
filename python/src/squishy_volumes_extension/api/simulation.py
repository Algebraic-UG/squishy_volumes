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

import uuid

import bpy

from ..bridge import SimulationHandle
from ..cache_paths import first_frame_file, simulation_input_exists
from ..capture_markers import (
    range_to_record,
    remove_capture_markers_if_enabled,
    set_capture_range,
)
from ..errors import SimulationNotFound, SquishyVolumesError
from ..get_preferences import get_default_compute_device
from ..squishy_volumes_properties import (
    PER_SIMULATION_PROPERTIES,
    get_input_objects_with_uuid,
    get_simulation_objects,
    stored_range,
    update_directory,
)
from ..util import copy_simple_property_group


def simulations() -> list[bpy.types.Object]:
    return get_simulation_objects()


def find_simulation(simulation_uuid: str) -> bpy.types.Object:
    candidates = [
        sim_obj
        for sim_obj in simulations()
        if sim_obj.squishy_volumes.uuid == simulation_uuid
    ]
    if len(candidates) != 1:
        raise SimulationNotFound(
            f"Expected one simulation with UUID '{simulation_uuid}', "
            f"found {len(candidates)}"
        )
    return candidates[0]


def add_simulation(
    name: str = "My Simulation",
    *,
    directory: str | None = None,
    scene: bpy.types.Scene | None = None,
) -> bpy.types.Object:
    scene = scene_or_current(scene)
    sim_obj = _new_simulation_object(name)
    sim_props = sim_obj.squishy_volumes
    sim_props.is_simulation = True
    sim_props.uuid = str(uuid.uuid4())
    sim_props.compute_device = get_default_compute_device()

    if directory is None:
        update_directory(sim_props, bpy.context)
    else:
        sim_props.directory = directory

    set_capture_range(scene, sim_obj, scene.frame_start, scene.frame_start)
    sim_props.cache_file = _new_cache_file(sim_obj)
    return sim_obj


def clone_simulation(
    sim_obj: bpy.types.Object, *, scene: bpy.types.Scene | None = None
) -> bpy.types.Object:
    scene = scene_or_current(scene)
    clone = add_simulation(sim_obj.name, scene=scene)
    copy_simple_property_group(
        sim_obj.squishy_volumes, clone.squishy_volumes, PER_SIMULATION_PROPERTIES
    )

    capture_range = range_to_record(scene, sim_obj) or stored_range(
        sim_obj.squishy_volumes
    )
    set_capture_range(scene, clone, *capture_range)

    for input_obj in input_objects(sim_obj):
        _copy_input_object(input_obj, clone)
    return clone


def remove_simulation(
    sim_obj: bpy.types.Object, *, scene: bpy.types.Scene | None = None
):
    scene = scene_or_current(scene)
    for input_obj in input_objects(sim_obj):
        input_obj.squishy_volumes.uuid = ""
    remove_capture_markers_if_enabled(scene, sim_obj)
    bpy.data.objects.remove(sim_obj)


def input_objects(sim_obj: bpy.types.Object) -> list[bpy.types.Object]:
    return get_input_objects_with_uuid(sim_obj.squishy_volumes.uuid)


def reload(sim_obj: bpy.types.Object):
    sim_props = sim_obj.squishy_volumes
    sim_handle = SimulationHandle.load(
        uuid=sim_props.uuid, directory=sim_props.directory
    )
    update_frame_driver(sim_props, sim_handle.capture_start_frame())


def reload_all() -> list[bpy.types.Object]:
    unloaded = [
        sim_obj
        for sim_obj in simulations()
        if not SimulationHandle.exists(uuid=sim_obj.squishy_volumes.uuid)
        and simulation_input_exists(sim_obj.squishy_volumes.directory)
    ]
    for sim_obj in unloaded:
        reload(sim_obj)
    return unloaded


def scene_or_current(scene: bpy.types.Scene | None) -> bpy.types.Scene:
    resolved = scene or bpy.context.scene
    if resolved is None:
        raise SquishyVolumesError("There is no scene to add the simulation to")
    return resolved


def update_frame_driver(sim_props, capture_start_frame: int):
    cache_file = sim_props.cache_file
    if cache_file is None or cache_file.animation_data is None:
        return
    frame_driver = cache_file.animation_data.drivers.find("frame")
    if frame_driver is not None:
        frame_driver.driver.expression = f"frame - {capture_start_frame}"


def _new_simulation_object(name: str) -> bpy.types.Object:
    sim_obj = bpy.data.objects.new(name=name, object_data=None)
    sim_obj.empty_display_type = "CUBE"
    sim_obj.lock_location = (True,) * 3
    sim_obj.lock_rotation = (True,) * 3
    sim_obj.lock_scale = (True,) * 3
    # It won't persist otherwise.
    # https://github.com/Algebraic-UG/squishy_volumes/issues/247
    sim_obj.use_fake_user = True
    return sim_obj


def _new_cache_file(sim_obj: bpy.types.Object) -> bpy.types.CacheFile:
    sim_props = sim_obj.squishy_volumes
    filepath = first_frame_file(sim_props.directory)
    existing = set(bpy.data.cache_files)
    # bpy.data.cache_files has no new(), only this operator creates one
    bpy.ops.cachefile.open(filepath=bpy.path.abspath(filepath))
    created = set(bpy.data.cache_files) - existing
    if len(created) != 1:
        raise RuntimeError("File cache creation failed")
    cache_file = created.pop()
    cache_file.filepath = filepath
    cache_file.is_sequence = True
    cache_file.override_frame = True
    driver = cache_file.driver_add("frame").driver
    driver.expression = f"frame - {sim_props.capture_start_frame}"
    return cache_file


def _copy_input_object(input_obj: bpy.types.Object, clone: bpy.types.Object):
    copy = input_obj.copy()
    copy.squishy_volumes.uuid = clone.squishy_volumes.uuid
    for collection in input_obj.users_collection:
        collection.objects.link(copy)
