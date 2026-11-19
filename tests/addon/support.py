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

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

import bpy

from .bootstrap import ADDON_MODULE, ensure_extension_installed


def addon():
    return sys.modules[ADDON_MODULE]


def api():
    return addon().api


def add_mesh_object(name: str) -> bpy.types.Object:
    obj = bpy.data.objects.new(name, bpy.data.meshes.new(name))
    bpy.context.scene.collection.objects.link(obj)
    return obj


def make_input(obj: bpy.types.Object, sim_obj: bpy.types.Object):
    obj.squishy_volumes.uuid = sim_obj.squishy_volumes.uuid


def marker_frames(sim_obj: bpy.types.Object) -> list[int]:
    return sorted(
        marker.frame
        for marker in bpy.context.scene.timeline_markers
        if marker.name == sim_obj.name
    )


def simulation_objects() -> list[bpy.types.Object]:
    return [obj for obj in bpy.data.objects if obj.squishy_volumes.is_simulation]


def input_objects(sim_obj: bpy.types.Object) -> list[bpy.types.Object]:
    uuid = sim_obj.squishy_volumes.uuid
    return [
        obj
        for obj in bpy.data.objects
        if obj.squishy_volumes.uuid == uuid and not obj.squishy_volumes.is_simulation
    ]


def cache_directory(sim_obj: bpy.types.Object) -> Path:
    return Path(bpy.path.abspath(sim_obj.squishy_volumes.directory))


def simulation_handle(sim_obj: bpy.types.Object):
    return addon().bridge.SimulationHandle.get(uuid=sim_obj.squishy_volumes.uuid)


class AddonTestCase(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        ensure_extension_installed()

    def setUp(self):
        self.work_dir = Path(tempfile.mkdtemp(prefix="squishy_volumes_test_"))
        bpy.ops.wm.read_homefile(use_empty=True)
        bpy.ops.wm.save_as_mainfile(filepath=str(self.work_dir / "test.blend"))

    def tearDown(self):
        bpy.ops.wm.read_homefile(use_empty=True)
        shutil.rmtree(self.work_dir)
