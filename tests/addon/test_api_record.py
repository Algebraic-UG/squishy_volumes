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

from .support import AddonTestCase, api, cache_directory, simulation_handle


def add_primitive(add_operator, **kwargs) -> bpy.types.Object:
    add_operator(**kwargs)
    return bpy.context.active_object


class RecordTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.cube = add_primitive(bpy.ops.mesh.primitive_cube_add, size=2)
        api().add_input(self.sim_obj, self.cube, api().PARTICLES)
        self.plane = add_primitive(
            bpy.ops.mesh.primitive_plane_add, size=10, location=(0, 0, -2)
        )
        api().add_input(self.sim_obj, self.plane, api().COLLIDER)
        api().set_capture_range(self.sim_obj, 1, 3)
        bpy.context.scene.frame_set(7)

    def frame_driver_expression(self) -> str:
        cache_file = self.sim_obj.squishy_volumes.cache_file
        return cache_file.animation_data.drivers.find("frame").driver.expression


class RecordInputTest(RecordTestCase):
    def test_records_every_frame_of_the_capture_range(self):
        self.assertEqual(api().record_input(self.sim_obj), 3)

        self.assertTrue(
            (cache_directory(self.sim_obj) / "simulation_input.bin").exists()
        )
        header = simulation_handle(self.sim_obj).input_header()
        self.assertEqual(set(header["objects"]), {"Cube", "Plane"})
        self.assertGreater(header["total_particles"], 0)

    def test_yields_the_number_of_recorded_frames(self):
        self.assertEqual(list(api().iter_record_input(self.sim_obj)), [1, 2, 3])

    def test_restores_the_current_frame(self):
        api().record_input(self.sim_obj)

        self.assertEqual(bpy.context.scene.frame_current, 7)

    def test_displays_frames_relative_to_the_capture_start(self):
        api().set_capture_range(self.sim_obj, 4, 6)

        api().record_input(self.sim_obj)

        sim_props = self.sim_obj.squishy_volumes
        self.assertEqual(
            (sim_props.capture_start_frame, sim_props.capture_end_frame), (4, 6)
        )
        self.assertEqual(self.frame_driver_expression(), "frame - 4")

    def test_recording_again_discards_baked_frames(self):
        sim_props = self.sim_obj.squishy_volumes
        sim_props.bake_frames = 2
        sim_props.compute_device = "CPU"
        bpy.ops.scene.squishy_volumes_record_input_to_cache(
            uuid=sim_props.uuid, start_baking=True
        )
        api().wait_until_baked(self.sim_obj, timeout_sec=300)
        self.assertEqual(simulation_handle(self.sim_obj).available_frames(), 2)

        api().record_input(self.sim_obj)

        self.assertEqual(simulation_handle(self.sim_obj).available_frames(), 0)

    def test_closing_the_recording_early_leaves_no_simulation(self):
        recording = api().iter_record_input(self.sim_obj)
        next(recording)

        recording.close()

        self.assertIsNone(simulation_handle(self.sim_obj))
        self.assertEqual(bpy.context.scene.frame_current, 7)
        self.assertEqual(api().record_input(self.sim_obj), 3)


class RecordErrorTest(RecordTestCase):
    def test_broken_markers_keep_the_previous_recording(self):
        api().record_input(self.sim_obj)
        scene = bpy.context.scene
        scene.timeline_markers.remove(scene.timeline_markers[0])

        with self.assertRaises(api().CaptureRangeError):
            api().record_input(self.sim_obj)

        self.assertIsNotNone(simulation_handle(self.sim_obj))

    def test_capture_range_must_not_end_before_it_starts(self):
        with self.assertRaises(api().CaptureRangeError):
            api().set_capture_range(self.sim_obj, 5, 4)

    def test_scaled_particles_are_rejected_until_fixed(self):
        self.cube.scale = (2.0, 2.0, 2.0)

        with self.assertRaises(api().ScaledInputError):
            api().record_input(self.sim_obj)

        self.assertIsNone(simulation_handle(self.sim_obj))
        self.assertEqual(bpy.context.scene.frame_current, 7)
        self.cube.scale = (1.0, 1.0, 1.0)
        self.assertEqual(api().record_input(self.sim_obj), 3)

    def test_more_than_sixteen_colliders_are_rejected(self):
        for index in range(16):
            collider = add_primitive(
                bpy.ops.mesh.primitive_plane_add, location=(index * 3, 0, -5)
            )
            api().add_input(self.sim_obj, collider, api().COLLIDER)

        with self.assertRaises(api().InvalidInput):
            api().record_input(self.sim_obj)

    def test_errors_are_squishy_volumes_errors(self):
        for error in (
            api().CaptureRangeError,
            api().ScaledInputError,
            api().UnresolvableCacheDirectory,
        ):
            self.assertTrue(issubclass(error, api().SquishyVolumesError))


class UnsavedFileRecordTest(AddonTestCase):
    def test_blend_relative_cache_needs_a_saved_file(self):
        bpy.ops.wm.read_homefile(use_empty=True)
        sim_obj = api().add_simulation("Jelly")
        cube = add_primitive(bpy.ops.mesh.primitive_cube_add, size=2)
        api().add_input(sim_obj, cube)

        with self.assertRaises(api().UnresolvableCacheDirectory):
            api().record_input(sim_obj)


class RecordOperatorTest(RecordTestCase):
    def test_records_without_a_window(self):
        bpy.ops.scene.squishy_volumes_record_input_to_cache(
            uuid=self.sim_obj.squishy_volumes.uuid
        )

        self.assertIsNotNone(simulation_handle(self.sim_obj))
        self.assertEqual(bpy.context.scene.frame_current, 7)

    def test_errors_raise_in_scripts(self):
        self.cube.scale = (2.0, 2.0, 2.0)

        with self.assertRaisesRegex(RuntimeError, "is scaled"):
            bpy.ops.scene.squishy_volumes_record_input_to_cache(
                uuid=self.sim_obj.squishy_volumes.uuid
            )
