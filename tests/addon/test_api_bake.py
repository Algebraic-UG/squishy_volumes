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
import time

import bpy

from .support import (
    AddonTestCase,
    addon,
    api,
    cache_directory,
    simulation_handle,
)

BAKE_TIMEOUT_SEC = 300


def add_primitive(add_operator, **kwargs) -> bpy.types.Object:
    add_operator(**kwargs)
    return bpy.context.active_object


class BakeTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.sim_obj.squishy_volumes.compute_device = "CPU"
        cube = add_primitive(bpy.ops.mesh.primitive_cube_add, size=2)
        api().add_input(self.sim_obj, cube, api().PARTICLES)
        plane = add_primitive(
            bpy.ops.mesh.primitive_plane_add, size=10, location=(0, 0, -2)
        )
        api().add_input(self.sim_obj, plane, api().COLLIDER)
        api().record_input(self.sim_obj)

    def bake_and_wait(self, **kwargs) -> int:
        api().bake(self.sim_obj, **kwargs)
        return api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def stored_frames(self) -> list[int]:
        return sorted(
            int(path.stem.removeprefix("frame_"))
            for path in cache_directory(self.sim_obj).glob("frame_*.abc")
        )

    def stats_frames(self) -> list[int]:
        directory = self.sim_obj.squishy_volumes.directory
        return sorted(addon().util.read_stats(directory))


class BakeTest(BakeTestCase):
    def test_bakes_the_requested_number_of_frames(self):
        self.sim_obj.squishy_volumes.bake_frames = 3

        self.assertEqual(self.bake_and_wait(), 3)

        self.assertEqual(api().baked_frames(self.sim_obj), 3)
        self.assertFalse(api().is_baking(self.sim_obj))
        self.assertEqual(self.stored_frames(), [0, 1, 2])
        self.assertEqual(self.stats_frames(), [0, 1, 2])

    def test_frames_overrides_the_bake_frames_setting(self):
        self.assertEqual(self.bake_and_wait(bake_frames=2), 2)

    def test_continues_from_the_latest_frame(self):
        self.bake_and_wait(bake_frames=2)

        self.assertEqual(self.bake_and_wait(bake_frames=4), 4)

        self.assertEqual(self.stored_frames(), [0, 1, 2, 3])
        self.assertEqual(self.stats_frames(), [0, 1, 2, 3])

    def test_rebaking_from_a_frame_replaces_the_frames_after_it(self):
        self.bake_and_wait(bake_frames=4)

        self.assertEqual(self.bake_and_wait(from_frame=0, bake_frames=2), 2)

        self.assertEqual(self.stored_frames(), [0, 1])
        self.assertEqual(self.stats_frames(), [0, 1])

    def test_baking_a_complete_cache_does_nothing(self):
        self.bake_and_wait(bake_frames=2)

        api().bake(self.sim_obj, bake_frames=2)

        self.assertFalse(api().is_baking(self.sim_obj))
        self.assertEqual(api().baked_frames(self.sim_obj), 2)

    def test_pause_stops_before_the_requested_frames(self):
        api().bake(self.sim_obj, bake_frames=10_000)
        self.assertTrue(api().is_baking(self.sim_obj))

        api().pause_bake(self.sim_obj)

        baked = api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        self.assertLess(baked, 10_000)
        self.assertFalse(api().is_baking(self.sim_obj))
        self.assertEqual(self.stored_frames(), list(range(baked)))

    def test_pausing_at_any_moment_leaves_no_errors_or_broken_frames(self):
        for delay_sec in (0.0, 0.01, 0.05, 0.2):
            with self.subTest(delay_sec=delay_sec):
                api().bake(self.sim_obj, bake_frames=10_000)
                time.sleep(delay_sec)
                api().pause_bake(self.sim_obj)

                baked = api().wait_until_baked(
                    self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC
                )
                self.assertEqual(api().bake_errors(self.sim_obj), [])
                self.assertEqual(self.bake_and_wait(bake_frames=baked + 1), baked + 1)

    def test_waiting_times_out(self):
        api().bake(self.sim_obj, bake_frames=10_000)

        with self.assertRaises(api().BakeTimeout):
            api().wait_until_baked(self.sim_obj, timeout_sec=0)

        api().pause_bake(self.sim_obj)
        api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)


class RebakeFromShownFrameTest(BakeTestCase):
    CAPTURE_START = 10

    def setUp(self):
        super().setUp()
        api().set_capture_range(self.sim_obj, self.CAPTURE_START, self.CAPTURE_START)
        api().record_input(self.sim_obj)
        self.sim_obj.squishy_volumes.bake_frames = 4
        self.bake_and_wait()

    def shown_frame(self, sim_obj: bpy.types.Object, scene_frame: int) -> float:
        bpy.context.scene.frame_set(scene_frame)
        return sim_obj.squishy_volumes.cache_file.frame

    def frame_modification_times(self, sim_obj: bpy.types.Object) -> list[int]:
        return [
            path.stat().st_mtime_ns
            for path in sorted(cache_directory(sim_obj).glob("frame_*.abc"))
        ]

    def rebake_from_scene_frame(self, sim_obj: bpy.types.Object, scene_frame: int):
        bpy.context.scene.frame_set(scene_frame)
        bpy.ops.scene.squishy_volumes_bake_start_from_loaded(
            uuid=sim_obj.squishy_volumes.uuid
        )
        api().wait_until_baked(sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def assert_rebaked_after(self, sim_obj: bpy.types.Object, frame: int):
        before = self.frame_modification_times(sim_obj)

        self.rebake_from_scene_frame(sim_obj, self.CAPTURE_START + frame)

        after = self.frame_modification_times(sim_obj)
        self.assertEqual(len(after), 4)
        for kept in range(frame + 1):
            self.assertEqual(after[kept], before[kept])
        for rebaked in range(frame + 1, 4):
            self.assertGreater(after[rebaked], before[rebaked])

    def test_shows_frames_relative_to_the_capture_start(self):
        self.assertEqual(self.shown_frame(self.sim_obj, self.CAPTURE_START + 1), 1)

    def test_rebakes_from_the_shown_frame(self):
        self.assert_rebaked_after(self.sim_obj, 1)

    def test_rebakes_from_the_recorded_start_after_the_capture_range_changed(self):
        api().set_capture_range(self.sim_obj, 1, 1)

        self.assertEqual(self.shown_frame(self.sim_obj, self.CAPTURE_START + 1), 1)
        self.assert_rebaked_after(self.sim_obj, 1)

    def test_a_reloaded_cache_lines_up_with_the_scene_frames(self):
        directory = self.sim_obj.squishy_volumes.directory
        simulation_handle(self.sim_obj).drop()
        api().remove_simulation(self.sim_obj)
        reloaded = api().add_simulation("Reloaded")
        reloaded.squishy_volumes.directory = directory
        reloaded.squishy_volumes.compute_device = "CPU"
        reloaded.squishy_volumes.bake_frames = 4

        api().reload(reloaded)

        self.assertEqual(self.shown_frame(reloaded, self.CAPTURE_START + 1), 1)
        self.assert_rebaked_after(reloaded, 1)


class BakeErrorTest(BakeTestCase):
    def test_rebaking_needs_an_existing_frame(self):
        self.bake_and_wait(bake_frames=2)

        for from_frame in (-1, 2):
            with self.assertRaises(api().InvalidBakeRange):
                api().bake(self.sim_obj, from_frame=from_frame)

        self.assertFalse(api().is_baking(self.sim_obj))

    def test_refuses_more_disk_space_than_is_free(self):
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1e12

        with self.assertRaises(api().InsufficientDiskSpace):
            api().bake(self.sim_obj, bake_frames=2)

        self.assertFalse(api().is_baking(self.sim_obj))

    def test_needs_a_loaded_simulation(self):
        unrecorded = api().add_simulation("Unrecorded")

        self.assertFalse(api().is_baking(unrecorded))
        self.assertEqual(api().baked_frames(unrecorded), 0)
        for action in (api().bake, api().pause_bake):
            with self.assertRaises(api().SimulationNotLoaded):
                action(unrecorded)
        with self.assertRaises(api().SimulationNotLoaded):
            api().wait_until_baked(unrecorded, timeout_sec=1)
        with self.assertRaises(api().SimulationNotLoaded):
            api().clear_bake_errors(unrecorded)
        self.assertEqual(api().bake_errors(unrecorded), [])

    def test_errors_are_squishy_volumes_errors(self):
        for error in (
            api().SimulationNotLoaded,
            api().InsufficientDiskSpace,
            api().InvalidBakeRange,
            api().BakeFailed,
            api().BakeTimeout,
        ):
            self.assertTrue(issubclass(error, api().SquishyVolumesError))


class BakeErrorListTest(BakeTestCase):
    def setUp(self):
        super().setUp()
        self.bake_and_wait(bake_frames=3)
        self.missing_frame = cache_directory(self.sim_obj) / "frame_00001.abc"
        self.kept_frame = self.missing_frame.with_suffix(".kept")
        shutil.move(self.missing_frame, self.kept_frame)

    def restore_missing_frame(self):
        shutil.move(self.kept_frame, self.missing_frame)

    def test_a_clean_bake_has_no_errors(self):
        self.restore_missing_frame()

        self.assertEqual(api().bake_errors(self.sim_obj), [])

    def test_waiting_fails_even_after_someone_else_polled(self):
        simulation_handle(self.sim_obj).poll()

        with self.assertRaisesRegex(api().BakeFailed, "missing"):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def test_errors_are_kept_until_cleared(self):
        [error] = api().bake_errors(self.sim_obj)
        self.restore_missing_frame()

        self.assertEqual(api().bake_errors(self.sim_obj), [error])
        api().clear_bake_errors(self.sim_obj)
        self.assertEqual(api().bake_errors(self.sim_obj), [])

    def test_starting_a_bake_clears_the_errors(self):
        api().bake_errors(self.sim_obj)
        self.restore_missing_frame()

        self.assertEqual(self.bake_and_wait(bake_frames=4), 4)
        self.assertEqual(api().bake_errors(self.sim_obj), [])


class ComputeFailureTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.preferences = addon().get_preferences.get_preferences()
        self.allowed_scaled_input = self.preferences.allow_scaled_input
        self.preferences.allow_scaled_input = True
        self.sim_obj = api().add_simulation("Mirrored")
        self.sim_obj.squishy_volumes.compute_device = "CPU"
        cube = add_primitive(bpy.ops.mesh.primitive_cube_add, size=2)
        cube.scale = (-1.0, 1.0, 1.0)
        api().add_input(self.sim_obj, cube, api().PARTICLES)
        plane = add_primitive(
            bpy.ops.mesh.primitive_plane_add, size=10, location=(0, 0, -2)
        )
        api().add_input(self.sim_obj, plane, api().COLLIDER)
        api().record_input(self.sim_obj)

    def tearDown(self):
        self.preferences.allow_scaled_input = self.allowed_scaled_input
        super().tearDown()

    def test_inverted_particles_fail_the_bake(self):
        api().bake(self.sim_obj, bake_frames=5)

        with self.assertRaisesRegex(api().BakeFailed, "Cpu compute error"):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def test_pausing_after_the_failure_reports_it(self):
        api().bake(self.sim_obj, bake_frames=5)
        sim_handle = simulation_handle(self.sim_obj)
        deadline = time.monotonic() + BAKE_TIMEOUT_SEC
        while sim_handle.baking() and time.monotonic() < deadline:
            time.sleep(0.01)

        api().pause_bake(self.sim_obj)

        [error] = sim_handle.bake_errors
        self.assertIn("Cpu compute error", error)
        self.assertEqual(api().bake_errors(self.sim_obj), [error])

    def test_a_repeated_failure_after_a_restart_is_unseen_again(self):
        unseen_bake_errors = addon().progress_update.unseen_bake_errors
        api().bake(self.sim_obj, bake_frames=5)
        with self.assertRaises(api().BakeFailed):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        sim_handle = simulation_handle(self.sim_obj)
        first_errors = list(sim_handle.bake_errors)
        shown = (sim_handle.bake_errors, len(sim_handle.bake_errors))

        api().bake(self.sim_obj, from_frame=0, bake_frames=5)
        with self.assertRaises(api().BakeFailed):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

        self.assertEqual(sim_handle.bake_errors, first_errors)
        self.assertEqual(
            unseen_bake_errors(sim_handle.bake_errors, shown), first_errors
        )

    def test_the_failure_survives_an_earlier_poll(self):
        api().bake(self.sim_obj, bake_frames=5)
        sim_handle = simulation_handle(self.sim_obj)
        deadline = time.monotonic() + BAKE_TIMEOUT_SEC
        while sim_handle.baking() and time.monotonic() < deadline:
            sim_handle.poll()
        sim_handle.poll()
        self.assertTrue(api().bake_errors(self.sim_obj))

        with self.assertRaisesRegex(api().BakeFailed, "Cpu compute error"):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)


class UnseenBakeErrorsTest(AddonTestCase):
    def unseen(self, errors, shown):
        return addon().progress_update.unseen_bake_errors(errors, shown)

    def test_everything_is_unseen_at_first(self):
        self.assertEqual(self.unseen(["broken"], None), ["broken"])

    def test_only_errors_added_since_are_unseen(self):
        errors = ["broken"]
        shown = (errors, len(errors))
        errors.append("worse")

        self.assertEqual(self.unseen(errors, shown), ["worse"])
        self.assertEqual(self.unseen(errors, (errors, len(errors))), [])

    def test_a_new_list_is_unseen_even_with_the_same_errors(self):
        shown = (["broken"], 1)

        self.assertEqual(self.unseen(["broken"], shown), ["broken"])


class BakeOperatorTest(BakeTestCase):
    def test_bake_and_pause(self):
        uuid = self.sim_obj.squishy_volumes.uuid
        self.sim_obj.squishy_volumes.bake_frames = 10_000

        bpy.ops.scene.squishy_volumes_bake_start_from_latest(uuid=uuid)
        self.assertTrue(api().is_baking(self.sim_obj))
        bpy.ops.scene.squishy_volumes_bake_pause(uuid=uuid)

        baked = api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        self.assertLess(baked, 10_000)

    def test_errors_raise_in_scripts(self):
        sim_props = self.sim_obj.squishy_volumes
        sim_props.bake_frames = 2
        sim_props.max_giga_bytes_on_disk = 1e12

        with self.assertRaisesRegex(RuntimeError, "run out of space"):
            bpy.ops.scene.squishy_volumes_bake_start_from_latest(uuid=sim_props.uuid)
