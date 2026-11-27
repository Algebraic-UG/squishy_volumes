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

import importlib
import json
import re
import shutil
import time
from types import SimpleNamespace
from unittest import mock

import bpy
from mathutils import Vector

from .support import (
    AddonTestCase,
    addon,
    api,
    cache_directory,
    simulation_handle,
)

BAKE_TIMEOUT_SEC = 300
INVERTED_PARTICLES_CAUSE = [
    "cpu computation failed",
    "failed to compute the elastic energy of a particle",
    "the position gradient has a negative determinant",
]


def message_lines(message: str) -> list[str]:
    lines = (line.strip() for line in message.splitlines())
    return [
        re.sub(r"^\d+: ", "", line) for line in lines if line and line != "Caused by:"
    ]


def add_primitive(add_operator, **kwargs) -> bpy.types.Object:
    add_operator(**kwargs)
    return bpy.context.active_object


class BakeTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.sim_obj.squishy_volumes.compute_device = "CPU"
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1.0
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


def attribute_rows(data, name: str) -> list[tuple[float, ...]]:
    attribute = data.attributes[name]
    width = {"INT": 1, "FLOAT2": 2, "FLOAT_VECTOR": 3}[attribute.data_type]
    key = "value" if width == 1 else "vector"
    values = [0] * (len(attribute.data) * width)
    attribute.data.foreach_get(key, values)
    return [tuple(values[i : i + width]) for i in range(0, len(values), width)]


class AnimatedGoalsBakeTest(BakeTestCase):
    def evaluated(self, obj: bpy.types.Object, scene_frame: int):
        bpy.context.scene.frame_set(scene_frame)
        return obj.evaluated_get(bpy.context.evaluated_depsgraph_get()).data

    def test_bakes_moving_goals_over_several_frames(self):
        cube = bpy.data.objects["Cube"]
        for name in ("Flags", "Goal Position"):
            cube.squishy_volumes.alembic_attributes[name].enabled = True
        choose, move = api().add_goals(cube)
        for frame, offset in ((1, 0.0), (3, 0.5)):
            choose.location.x = 2 * offset
            choose.keyframe_insert("location", frame=frame)
            move.location.z = offset
            move.keyframe_insert("location", frame=frame)
        api().set_capture_range(self.sim_obj, 1, 3)
        api().record_input(self.sim_obj)

        self.assertEqual(self.bake_and_wait(bake_frames=3), 3)

        self.assertEqual(api().bake_errors(self.sim_obj), [])
        output_attributes = importlib.import_module(
            f"{addon().__name__}.output_attributes"
        )
        has_goal_bit = 1 << output_attributes.PARTICLE_FLAG_BITS["HAS_GOAL"]
        captured = self.evaluated(cube, 3)
        has_goal = [
            bool(flags & has_goal_bit)
            for (flags,) in attribute_rows(captured, "sv_flags")
        ]
        goals = attribute_rows(captured, "sv_goal_position")
        output_obj = bpy.data.objects["Cube - Output"]
        output_obj.modifiers["Squishy Volumes Output"].show_viewport = False
        baked = self.evaluated(output_obj, 3)
        baked_has_goal = [
            bool(int(low) & has_goal_bit)
            for low, _high in attribute_rows(baked, "flags")
        ]
        baked_goals = [
            tuple(output_obj.matrix_world @ Vector(goal))
            for goal in attribute_rows(baked, "goal_position")
        ]

        self.assertTrue(any(has_goal))
        self.assertFalse(all(has_goal))
        self.assertEqual(baked_has_goal, has_goal)
        for index in (index for index, goal in enumerate(has_goal) if goal):
            for found, expected in zip(baked_goals[index], goals[index], strict=True):
                self.assertAlmostEqual(found, expected, places=4, msg=index)


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
        reloaded.squishy_volumes.max_giga_bytes_on_disk = 1.0
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
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1e9

        with self.assertRaises(api().InsufficientDiskSpace) as refusal:
            api().bake(self.sim_obj, bake_frames=2)

        self.assertIn("GB are free on the disk", str(refusal.exception))
        self.assertEqual(refusal.exception.error_code, "insufficient_disk_space")
        self.assertFalse(api().is_baking(self.sim_obj))

    def test_a_core_error_without_a_code_is_a_plain_runtime_error(self):
        wrapped_simulation = type(simulation_handle(self.sim_obj).handle)
        missing_directory = cache_directory(self.sim_obj) / "missing"

        with self.assertRaises(RuntimeError) as failure:
            wrapped_simulation.load(uuid="missing", directory=str(missing_directory))

        self.assertIsNone(failure.exception.error_code)
        with self.assertRaises(AttributeError):
            failure.exception.error_code = "insufficient_disk_space"

    def raw_disk_space_error(self) -> RuntimeError:
        sim_handle = simulation_handle(self.sim_obj)
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1e9
        with mock.patch.object(type(sim_handle), "start_compute") as start_compute:
            api().bake(self.sim_obj, bake_frames=2)
        settings = start_compute.call_args.kwargs["compute_settings"]
        with self.assertRaises(RuntimeError) as raw:
            sim_handle.handle.start_compute(compute_settings=json.dumps(settings))
        return raw.exception

    def fail_polling_with(self, error: RuntimeError):
        sim_handle = simulation_handle(self.sim_obj)
        self.addCleanup(setattr, sim_handle, "handle", sim_handle.handle)
        sim_handle.handle = SimpleNamespace(poll=mock.Mock(side_effect=error))

    def test_a_disk_space_failure_while_baking_keeps_its_class(self):
        self.fail_polling_with(self.raw_disk_space_error())

        [error] = api().bake_errors(self.sim_obj)

        self.assertIsInstance(error, api().InsufficientDiskSpace)
        self.assertEqual(error.error_code, "insufficient_disk_space")
        self.assertIn("GB are free on the disk", str(error))

    def test_a_disk_space_failure_while_baking_offers_to_disable_the_check(self):
        self.fail_polling_with(self.raw_disk_space_error())
        progress_update = addon().progress_update

        with mock.patch.object(progress_update, "show_message") as show_message:
            progress_update.update_progress()

        [(_uuid, _message, preference)] = [c.args for c in show_message.call_args_list]
        self.assertEqual(preference, "sanity_check_allowed_disk_space")

    def test_bakes_beyond_the_free_space_with_the_check_disabled(self):
        preferences = addon().get_preferences.get_preferences()
        preferences.sanity_check_allowed_disk_space = False
        self.addCleanup(setattr, preferences, "sanity_check_allowed_disk_space", True)
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1e9

        self.assertEqual(self.bake_and_wait(bake_frames=2), 2)

    def test_bakes_with_a_limit_beyond_what_bytes_can_count(self):
        preferences = addon().get_preferences.get_preferences()
        preferences.sanity_check_allowed_disk_space = False
        self.addCleanup(setattr, preferences, "sanity_check_allowed_disk_space", True)
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1e12

        self.assertEqual(self.bake_and_wait(bake_frames=2), 2)

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

        with self.assertRaisesRegex(api().BakeFailed, "frame_00001.abc.* missing"):
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
        self.sim_obj.squishy_volumes.max_giga_bytes_on_disk = 1.0
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

        with self.assertRaisesRegex(api().BakeFailed, "cpu computation failed"):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def fail_the_bake(self, **kwargs) -> str:
        api().bake(self.sim_obj, bake_frames=5, **kwargs)
        with self.assertRaises(api().BakeFailed) as failure:
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        [error] = api().bake_errors(self.sim_obj)
        self.assertIn(str(error), str(failure.exception))
        return str(error)

    def message_shown_to_the_user(self) -> str:
        addon().progress_update.update_progress()
        return simulation_handle(self.sim_obj).last_error

    def assert_readable(self, message: str):
        self.assertNotRegex(message, r"\b[A-Z]\w* ?[({]")
        self.assertNotRegex(message, r"(?i)traceback|backtrace|panicked")

    def test_a_live_failure_names_its_cause(self):
        error = self.fail_the_bake()

        self.assertEqual(message_lines(error), INVERTED_PARTICLES_CAUSE)

    def test_rebaking_from_the_failed_frame_repeats_its_cause(self):
        self.fail_the_bake()
        failed_frame = api().baked_frames(self.sim_obj) - 1

        error = self.fail_the_bake()

        self.assertEqual(
            message_lines(error),
            [
                f"frame {failed_frame} failed, rebake from an earlier frame to retry",
                *INVERTED_PARTICLES_CAUSE,
            ],
        )

    def test_the_failures_shown_to_the_user_are_readable(self):
        live_error = self.fail_the_bake()
        shown_live = self.message_shown_to_the_user()
        stored_error = self.fail_the_bake()
        shown_stored = self.message_shown_to_the_user()

        self.assertIn(live_error, shown_live)
        self.assertIn(stored_error, shown_stored)
        for message in [shown_live, shown_stored]:
            self.assert_readable(message)

    def test_pausing_after_the_failure_reports_it(self):
        api().bake(self.sim_obj, bake_frames=5)
        sim_handle = simulation_handle(self.sim_obj)
        deadline = time.monotonic() + BAKE_TIMEOUT_SEC
        while sim_handle.baking() and time.monotonic() < deadline:
            time.sleep(0.01)

        api().pause_bake(self.sim_obj)

        [error] = sim_handle.bake_errors
        self.assertIn("cpu computation failed", str(error))
        self.assertEqual(api().bake_errors(self.sim_obj), [error])

    def test_a_repeated_failure_after_a_restart_is_unseen_again(self):
        unseen_bake_errors = addon().progress_update.unseen_bake_errors
        api().bake(self.sim_obj, bake_frames=5)
        with self.assertRaises(api().BakeFailed):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        sim_handle = simulation_handle(self.sim_obj)
        first_errors = list(map(str, sim_handle.bake_errors))
        shown = (sim_handle.bake_errors, len(sim_handle.bake_errors))

        api().bake(self.sim_obj, from_frame=0, bake_frames=5)
        with self.assertRaises(api().BakeFailed):
            api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

        self.assertEqual(list(map(str, sim_handle.bake_errors)), first_errors)
        self.assertEqual(
            list(map(str, unseen_bake_errors(sim_handle.bake_errors, shown))),
            first_errors,
        )

    def test_the_failure_survives_an_earlier_poll(self):
        api().bake(self.sim_obj, bake_frames=5)
        sim_handle = simulation_handle(self.sim_obj)
        deadline = time.monotonic() + BAKE_TIMEOUT_SEC
        while sim_handle.baking() and time.monotonic() < deadline:
            sim_handle.poll()
        sim_handle.poll()
        self.assertTrue(api().bake_errors(self.sim_obj))

        with self.assertRaisesRegex(api().BakeFailed, "cpu computation failed"):
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
        sim_props.max_giga_bytes_on_disk = 1e9

        operator_base = addon().operators.operator_base
        with (
            mock.patch.object(operator_base, "popup") as popup,
            self.assertRaisesRegex(RuntimeError, "GB are free on the disk"),
        ):
            bpy.ops.scene.squishy_volumes_bake_start_from_latest(uuid=sim_props.uuid)

        [(_uuid, message, preference)] = [c.args for c in popup.call_args_list]
        self.assertIn("GB are free on the disk", message)
        self.assertEqual(preference, "sanity_check_allowed_disk_space")
