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

from pathlib import Path

import bpy

from .support import AddonTestCase, addon, api


def preferences():
    return addon().get_preferences.get_preferences()


def first_run_setup_pending() -> bool:
    return addon().get_preferences.first_run_setup_pending(preferences())


def detected_device_identifiers() -> list[str]:
    return [device[0] for device in addon().bridge.DETECTED_DEVICES]


class FirstRunSetupTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.blender_preferences = bpy.context.preferences
        self.use_preferences_save = self.blender_preferences.use_preferences_save
        self.device_identifier = preferences().default_compute_device_identifier
        self.cache_location = preferences().default_cache_location

    def tearDown(self):
        self.blender_preferences.use_preferences_save = self.use_preferences_save
        preferences().first_run_setup_done = False
        preferences().default_compute_device_identifier = self.device_identifier
        preferences().default_cache_location = self.cache_location
        super().tearDown()

    def test_setup_is_pending_by_default(self):
        self.assertFalse(preferences().first_run_setup_done)
        self.assertTrue(first_run_setup_pending())

    def test_setup_is_not_pending_once_done(self):
        preferences().first_run_setup_done = True

        self.assertFalse(first_run_setup_pending())

    def test_api_works_while_setup_is_pending(self):
        sim_obj = api().add_simulation()

        self.assertTrue(first_run_setup_pending())
        self.assertEqual(sim_obj.squishy_volumes.compute_device, "CPU")
        self.assertEqual(sim_obj.squishy_volumes.directory[:2], "//")

    def test_add_simulation_uses_the_default_device(self):
        last_device = detected_device_identifiers()[-1]
        preferences().default_compute_device = last_device

        sim_obj = api().add_simulation()

        self.assertEqual(sim_obj.squishy_volumes.compute_device, last_device)

    def test_undetected_default_device_falls_back_to_the_first(self):
        preferences().default_compute_device_identifier = "Gone GPU"

        sim_obj = api().add_simulation()

        self.assertEqual(
            sim_obj.squishy_volumes.compute_device, detected_device_identifiers()[0]
        )

    def test_add_simulation_uses_the_default_cache_location(self):
        preferences().default_cache_location = "//elsewhere"

        sim_obj = api().add_simulation()

        self.assertTrue(sim_obj.squishy_volumes.directory.startswith("//elsewhere"))

    def test_done_with_auto_save_marks_preferences_for_saving(self):
        self.blender_preferences.use_preferences_save = True
        self.blender_preferences.is_dirty = False

        bpy.ops.scene.squishy_volumes_finish_first_run_setup()

        self.assertFalse(first_run_setup_pending())
        self.assertTrue(self.blender_preferences.is_dirty)

    def test_done_without_auto_save_saves_preferences(self):
        self.blender_preferences.use_preferences_save = False
        userpref = Path(bpy.utils.resource_path("USER")) / "config" / "userpref.blend"
        userpref.unlink(missing_ok=True)

        bpy.ops.scene.squishy_volumes_finish_first_run_setup()

        self.assertFalse(first_run_setup_pending())
        self.assertTrue(userpref.exists())

    def test_done_is_unavailable_once_done(self):
        preferences().first_run_setup_done = True

        self.assertFalse(bpy.ops.scene.squishy_volumes_finish_first_run_setup.poll())
