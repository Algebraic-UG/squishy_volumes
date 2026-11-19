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

from .support import (
    AddonTestCase,
    addon,
    cache_directory,
    input_objects,
    simulation_handle,
    simulation_objects,
)


class BoingBlockExampleTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        bpy.ops.scene.squishy_volumes_add_example_simulation(
            startup_choice="Boing Block"
        )
        [self.sim_obj] = simulation_objects()

    def test_records_the_cube_and_the_plane(self):
        self.assertCountEqual(
            [obj.name for obj in input_objects(self.sim_obj)], ["Cube", "Plane"]
        )
        self.assertTrue(
            (cache_directory(self.sim_obj) / "simulation_input.bin").exists()
        )
        self.assertIn("Cube - Output", bpy.data.objects)

    def test_bake_provides_all_requested_frames(self):
        sim_props = self.sim_obj.squishy_volumes
        sim_props.bake_frames = 3
        sim_props.compute_device = "CPU"

        bpy.ops.scene.squishy_volumes_bake_start_from_latest(uuid=sim_props.uuid)
        bpy.ops.scene.squishy_volumes_wait_until_finished(
            simulation_uuid=sim_props.uuid, timeout_sec=300
        )

        sim_handle = simulation_handle(self.sim_obj)
        sim_handle.poll()
        self.assertIsNone(sim_handle.last_error)
        self.assertEqual(sim_handle.available_frames(), 3)
        stats = addon().util.read_stats(sim_props.directory)
        self.assertEqual(sorted(stats), [0, 1, 2])


class BenchmarkExampleTest(AddonTestCase):
    def test_records_the_monkeys_and_colliders(self):
        bpy.ops.scene.squishy_volumes_add_example_simulation(startup_choice="Benchmark")
        [sim_obj] = simulation_objects()

        input_types = [obj.squishy_volumes.input_type for obj in input_objects(sim_obj)]
        self.assertEqual(input_types.count("Particles"), 8)
        self.assertEqual(input_types.count("Collider"), 3)
        self.assertTrue((cache_directory(sim_obj) / "simulation_input.bin").exists())
