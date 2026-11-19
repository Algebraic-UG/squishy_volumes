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
    add_mesh_object,
    api,
    cache_directory,
    make_input,
    marker_frames,
    simulation_handle,
)


class AddSimulationTest(AddonTestCase):
    def test_creates_a_simulation_object(self):
        sim_obj = api().add_simulation("Jelly")

        self.assertEqual(sim_obj.name, "Jelly")
        self.assertTrue(sim_obj.squishy_volumes.is_simulation)
        self.assertNotEqual(sim_obj.squishy_volumes.uuid, "")
        self.assertEqual(sim_obj.empty_display_type, "CUBE")
        self.assertTrue(sim_obj.use_fake_user)
        self.assertEqual(api().simulations(), [sim_obj])

    def test_starts_recording_at_the_scene_start(self):
        bpy.context.scene.frame_start = 11

        sim_obj = api().add_simulation()

        sim_props = sim_obj.squishy_volumes
        self.assertEqual(
            (sim_props.capture_start_frame, sim_props.capture_end_frame), (11, 11)
        )
        self.assertEqual(marker_frames(sim_obj), [11, 11])

    def test_cache_file_follows_the_directory_and_start_frame(self):
        sim_obj = api().add_simulation(directory="//jelly_cache/")

        sim_props = sim_obj.squishy_volumes
        self.assertEqual(sim_props.directory, "//jelly_cache")
        cache_file = sim_props.cache_file
        self.assertEqual(cache_file.filepath, "//jelly_cache/frame_00000.abc")
        self.assertTrue(cache_file.is_sequence)
        self.assertTrue(cache_file.override_frame)
        [frame_driver] = cache_file.animation_data.drivers
        self.assertEqual(
            frame_driver.driver.expression,
            f"frame - {sim_props.capture_start_frame}",
        )

    def test_does_not_write_to_the_cache_directory(self):
        sim_obj = api().add_simulation()

        self.assertFalse(cache_directory(sim_obj).exists())

    def test_two_simulations_get_different_uuids_and_directories(self):
        first = api().add_simulation()
        second = api().add_simulation()

        self.assertNotEqual(first.squishy_volumes.uuid, second.squishy_volumes.uuid)
        self.assertNotEqual(
            first.squishy_volumes.directory, second.squishy_volumes.directory
        )


class FindSimulationTest(AddonTestCase):
    def test_finds_by_uuid(self):
        sim_obj = api().add_simulation()

        self.assertEqual(api().find_simulation(sim_obj.squishy_volumes.uuid), sim_obj)

    def test_unknown_uuid_raises(self):
        api().add_simulation()

        with self.assertRaises(api().SimulationNotFound):
            api().find_simulation("not a simulation")

    def test_errors_are_runtime_errors(self):
        self.assertTrue(issubclass(api().SimulationNotFound, RuntimeError))


class CloneSimulationTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.source = api().add_simulation("Source")
        source_props = self.source.squishy_volumes
        source_props.grid_node_size = 0.3
        source_props.time_step = 0.002
        source_props.bake_frames = 7
        source_props.gravity = (0.0, 0.0, -1.0)
        for marker, frame in zip(
            bpy.context.scene.timeline_markers, (5, 20), strict=True
        ):
            marker.frame = frame
        self.source_input = add_mesh_object("Ball")
        make_input(self.source_input, self.source)

    def test_copies_the_settings(self):
        clone = api().clone_simulation(self.source)

        clone_props = clone.squishy_volumes
        self.assertAlmostEqual(clone_props.grid_node_size, 0.3)
        self.assertAlmostEqual(clone_props.time_step, 0.002)
        self.assertEqual(clone_props.bake_frames, 7)
        self.assertEqual(tuple(clone_props.gravity), (0.0, 0.0, -1.0))

    def test_gets_its_own_identity_and_cache(self):
        clone = api().clone_simulation(self.source)

        self.assertNotEqual(
            clone.squishy_volumes.uuid, self.source.squishy_volumes.uuid
        )
        self.assertNotEqual(
            clone.squishy_volumes.directory, self.source.squishy_volumes.directory
        )
        self.assertNotEqual(
            clone.squishy_volumes.cache_file, self.source.squishy_volumes.cache_file
        )

    def test_records_the_same_frames(self):
        clone = api().clone_simulation(self.source)

        self.assertEqual(marker_frames(clone), [5, 20])
        clone_props = clone.squishy_volumes
        self.assertEqual(
            (clone_props.capture_start_frame, clone_props.capture_end_frame), (5, 20)
        )

    def test_copies_the_inputs_next_to_the_originals(self):
        clone = api().clone_simulation(self.source)

        [clone_input] = api().input_objects(clone)
        self.assertNotEqual(clone_input, self.source_input)
        self.assertEqual(
            list(clone_input.users_collection), list(self.source_input.users_collection)
        )
        self.assertEqual(api().input_objects(self.source), [self.source_input])


class RemoveSimulationTest(AddonTestCase):
    def test_releases_the_inputs_and_markers(self):
        sim_obj = api().add_simulation("Doomed")
        input_obj = add_mesh_object("Ball")
        make_input(input_obj, sim_obj)

        api().remove_simulation(sim_obj)

        self.assertEqual(api().simulations(), [])
        self.assertEqual(input_obj.squishy_volumes.uuid, "")
        self.assertEqual(len(bpy.context.scene.timeline_markers), 0)


class ReloadTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        bpy.ops.scene.squishy_volumes_add_example_simulation(
            startup_choice="Boing Block"
        )
        [self.sim_obj] = api().simulations()
        simulation_handle(self.sim_obj).drop()

    def test_reload_opens_the_recorded_cache(self):
        api().reload(self.sim_obj)

        sim_handle = simulation_handle(self.sim_obj)
        self.assertIsNotNone(sim_handle)
        self.assertGreater(sim_handle.input_header()["total_particles"], 0)

    def test_reload_all_opens_only_unloaded_caches(self):
        self.assertEqual(api().reload_all(), [self.sim_obj])
        self.assertIsNotNone(simulation_handle(self.sim_obj))
        self.assertEqual(api().reload_all(), [])

    def test_reload_without_a_cache_raises(self):
        empty = api().add_simulation()

        with self.assertRaises(RuntimeError):
            api().reload(empty)


class SimulationOperatorTest(AddonTestCase):
    def test_add_selects_the_new_simulation(self):
        bpy.ops.scene.squishy_volumes_add_simulation(sim_name="Jelly")

        [sim_obj] = api().simulations()
        self.assertEqual(sim_obj.name, "Jelly")
        self.assertEqual(bpy.context.scene.squishy_volumes.selected_simulation, sim_obj)

    def test_clone_selects_the_clone(self):
        source = api().add_simulation("Source")

        bpy.ops.scene.squishy_volumes_clone_simulation(uuid=source.squishy_volumes.uuid)

        [clone] = [sim_obj for sim_obj in api().simulations() if sim_obj != source]
        self.assertEqual(bpy.context.scene.squishy_volumes.selected_simulation, clone)

    def test_errors_raise_in_scripts(self):
        with self.assertRaisesRegex(RuntimeError, "not a simulation"):
            bpy.ops.scene.squishy_volumes_remove_simulation(uuid="not a simulation")

    def test_reload_error_raises_in_scripts(self):
        sim_obj = api().add_simulation()

        with self.assertRaises(RuntimeError):
            bpy.ops.scene.squishy_volumes_reload(uuid=sim_obj.squishy_volumes.uuid)
