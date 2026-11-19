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

import sys
import unittest
from pathlib import Path

import bpy

TESTS_DIR = Path(__file__).resolve().parent


def script_arguments() -> list[str]:
    return sys.argv[sys.argv.index("--") + 1 :]


def install_extension(zip_path: str):
    from support import ADDON_MODULE

    bpy.ops.extensions.package_install_files(
        repo="user_default", filepath=zip_path, enable_on_install=True
    )
    if ADDON_MODULE not in bpy.context.preferences.addons:
        sys.exit(f"Installing {zip_path} did not enable {ADDON_MODULE}")


def discover_tests(patterns: list[str]) -> unittest.TestSuite:
    loader = unittest.TestLoader()
    if patterns:
        loader.testNamePatterns = [f"*{pattern}*" for pattern in patterns]
    return loader.discover(str(TESTS_DIR))


def main():
    zip_path, *patterns = script_arguments()
    sys.path.insert(0, str(TESTS_DIR))
    install_extension(zip_path)
    result = unittest.TextTestRunner(verbosity=2).run(discover_tests(patterns))
    sys.exit(0 if result.wasSuccessful() else 1)


main()
