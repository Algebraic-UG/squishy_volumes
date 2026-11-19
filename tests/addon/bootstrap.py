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

import atexit
import os
import shutil
import sys
import tempfile
from pathlib import Path

ADDON_MODULE = "bl_ext.user_default.squishy_volumes_extension"
ZIP_VARIABLE = "SQUISHY_VOLUMES_ZIP"
EXTENSION_DIR = (
    Path(__file__).resolve().parents[2] / "python" / "src" / "squishy_volumes_extension"
)


def isolate_blender_user_resources():
    user_dir = tempfile.mkdtemp(prefix="squishy_volumes_user_")
    atexit.register(shutil.rmtree, user_dir, ignore_errors=True)
    os.environ["BLENDER_USER_RESOURCES"] = user_dir


def extension_zip() -> Path:
    if ZIP_VARIABLE in os.environ:
        return Path(os.environ[ZIP_VARIABLE])
    zips = sorted(
        EXTENSION_DIR.glob("squishy_volumes_extension-*.zip"),
        key=lambda zip_path: zip_path.stat().st_mtime,
    )
    if not zips:
        raise RuntimeError(
            f"No squishy_volumes_extension-*.zip in {EXTENSION_DIR}. "
            f"Build one (see README, 'Add-on Tests') or set {ZIP_VARIABLE}."
        )
    return zips[-1]


def ensure_extension_installed():
    # bpy as a module reads BLENDER_USER_RESOURCES when first imported
    import bpy

    if ADDON_MODULE in bpy.context.preferences.addons:
        return
    zip_path = extension_zip()
    bpy.ops.extensions.package_install_files(
        repo="user_default", filepath=str(zip_path), enable_on_install=True
    )
    if ADDON_MODULE not in bpy.context.preferences.addons:
        raise RuntimeError(f"Installing {zip_path} did not enable {ADDON_MODULE}")


def prepare_bpy_module():
    if "bpy" not in sys.modules:
        isolate_blender_user_resources()
