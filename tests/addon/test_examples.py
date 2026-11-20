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
from types import SimpleNamespace
from unittest import mock

import bpy

from .support import (
    AddonTestCase,
    addon,
    cache_directory,
    input_objects,
    simulation_handle,
    simulation_objects,
)

BAKE_TIMEOUT_SEC = 300


class ExampleTestCase(AddonTestCase):
    example = ""

    def setUp(self):
        super().setUp()
        bpy.ops.scene.squishy_volumes_add_example_simulation(
            startup_choice=self.example
        )
        [self.sim_obj] = simulation_objects()
        self.baking_after_setup = addon().api.is_baking(self.sim_obj)
        self.baked_after_setup = addon().api.baked_frames(self.sim_obj)

    def tearDown(self):
        addon().api.pause_bake(self.sim_obj)
        addon().api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        super().tearDown()

    def assert_started_baking(self):
        self.assertTrue(self.baking_after_setup or self.baked_after_setup > 0)


class BoingBlockExampleTest(ExampleTestCase):
    example = "Boing Block"

    def test_records_the_cube_and_the_plane(self):
        self.assertCountEqual(
            [obj.name for obj in input_objects(self.sim_obj)], ["Cube", "Plane"]
        )
        self.assertTrue(
            (cache_directory(self.sim_obj) / "simulation_input.bin").exists()
        )
        self.assertIn("Cube - Output", bpy.data.objects)

    def test_starts_baking(self):
        self.assert_started_baking()

    def test_rebake_provides_all_requested_frames(self):
        api = addon().api
        deadline = time.monotonic() + BAKE_TIMEOUT_SEC
        while api.baked_frames(self.sim_obj) < 1 and time.monotonic() < deadline:
            time.sleep(0.01)
        api.pause_bake(self.sim_obj)
        api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        self.sim_obj.squishy_volumes.compute_device = "CPU"

        api.bake(self.sim_obj, from_frame=0, bake_frames=3)

        self.assertEqual(
            api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC), 3
        )
        sim_handle = simulation_handle(self.sim_obj)
        self.assertIsNone(sim_handle.last_error)
        self.assertEqual(api.bake_errors(self.sim_obj), [])
        stats = addon().util.read_stats(self.sim_obj.squishy_volumes.directory)
        self.assertEqual(sorted(stats), [0, 1, 2])


class BenchmarkExampleTest(ExampleTestCase):
    example = "Benchmark"

    def test_records_the_monkeys_and_colliders(self):
        input_types = [
            obj.squishy_volumes.input_type for obj in input_objects(self.sim_obj)
        ]
        self.assertEqual(input_types.count("Particles"), 8)
        self.assertEqual(input_types.count("Collider"), 3)
        self.assertTrue(
            (cache_directory(self.sim_obj) / "simulation_input.bin").exists()
        )

    def test_starts_baking(self):
        self.assert_started_baking()


class ExampleWithoutDiskSpaceTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.preferences = addon().get_preferences._get_preferences()
        self.checked_disk_space = self.preferences.sanity_check_allowed_disk_space
        self.preferences.sanity_check_allowed_disk_space = True

    def tearDown(self):
        self.preferences.sanity_check_allowed_disk_space = self.checked_disk_space
        super().tearDown()

    def test_setup_finishes_without_baking(self):
        with mock.patch("shutil.disk_usage", return_value=SimpleNamespace(free=0)):
            result = bpy.ops.scene.squishy_volumes_add_example_simulation(
                startup_choice="Boing Block"
            )

        self.assertEqual(result, {"FINISHED"})
        [sim_obj] = simulation_objects()
        self.assertEqual(bpy.context.scene.squishy_volumes.selected_simulation, sim_obj)
        self.assertFalse(addon().api.is_baking(sim_obj))
        self.assertEqual(addon().api.baked_frames(sim_obj), 0)
        self.assertIn("run out of space", simulation_handle(sim_obj).last_error)


class ExampleWithFailedRecordingTest(AddonTestCase):
    def test_setup_finishes_without_recording(self):
        api = addon().api
        failure = api.CaptureRangeError("no frames to record")
        with mock.patch.object(api, "record_input", side_effect=failure):
            result = bpy.ops.scene.squishy_volumes_add_example_simulation(
                startup_choice="Boing Block"
            )

        self.assertEqual(result, {"FINISHED"})
        [sim_obj] = simulation_objects()
        self.assertEqual(bpy.context.scene.squishy_volumes.selected_simulation, sim_obj)
        self.assertCountEqual(
            [obj.name for obj in input_objects(sim_obj)], ["Cube", "Plane"]
        )
        self.assertIsNone(simulation_handle(sim_obj))


class ExampleInUnsavedFileTest(AddonTestCase):
    def test_refuses_before_changing_anything(self):
        bpy.ops.wm.read_homefile(use_empty=True)

        with self.assertRaisesRegex(RuntimeError, "Save the .blend file first"):
            bpy.ops.scene.squishy_volumes_add_example_simulation(
                startup_choice="Boing Block"
            )

        self.assertEqual(list(bpy.data.objects), [])
        self.assertEqual(list(bpy.data.collections), [])
        self.assertEqual(list(bpy.data.meshes), [])
        self.assertEqual(list(bpy.data.cache_files), [])
