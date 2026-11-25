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
import sys

import bpy

from .bootstrap import ADDON_MODULE
from .support import AddonTestCase, api

ALIAS = "squishy_volumes"


class ScriptingAliasTest(AddonTestCase):
    def test_imports_the_api(self):
        import squishy_volumes

        self.assertIs(squishy_volumes, api())
        sim_obj = squishy_volumes.add_simulation("Jelly")
        self.assertEqual(api().simulations(), [sim_obj])

    def test_unregister_removes_the_alias(self):
        self.addCleanup(bpy.ops.preferences.addon_enable, module=ADDON_MODULE)
        bpy.ops.preferences.addon_disable(module=ADDON_MODULE)

        self.assertNotIn(ALIAS, sys.modules)
        with self.assertRaises(ModuleNotFoundError):
            importlib.import_module(ALIAS)
