# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import bpy
import os
import argparse
import logging
import urllib
from pathlib import Path

from test_util import (
    extension_install,
    extension_repo_add,
    extension_repo_remove,
    extension_repo_update_url,
    extension_url,
    fetch_available_versions,
    installed_addons,
    test_factory_clean,
)


def test_install(remote_url):
    extension_repo_add(remote_url)
    extension_install(remote_url)
    assert len(installed_addons()) == 1


def test_uninstall(remote_url):
    extension_repo_remove(remote_url)
    test_factory_clean()


def test_update(url_old, url_new):
    extension_repo_update_url(url_old, url_new)
    extension_install(url_new)


if __name__ == "__main__":
    logging.getLogger().setLevel(logging.DEBUG)

    versions = fetch_available_versions()

    parser = argparse.ArgumentParser()
    parser.add_argument("--local-json")
    parser.add_argument("--version", choices=versions)

    args = parser.parse_args()

    if args.local_json:
        path = Path(args.local_json).expanduser().resolve()
        file_url = urllib.parse.urljoin("file:", urllib.request.pathname2url(str(path)))
        URL = file_url
    else:
        URL = extension_url(args.version)

    test_name = Path(__file__).stem
    test_dir = test_name + os.sep

    test_factory_clean()

    bpy.context.preferences.system.use_online_access = True

    test_install(URL)

    bpy.ops.scene.squishy_volumes_add_simulation(cache_directory=test_dir)
    simulation_uuid = bpy.context.scene.squishy_volumes_scene.simulations[0].uuid

    default_cube = bpy.data.objects["Cube"]
    default_cube.hide_render = True

    default_cube.select_set(True)
    bpy.context.view_layer.objects.active = default_cube
    bpy.ops.object.squishy_volumes_add_input_object()

    bpy.ops.mesh.primitive_plane_add(
        size=6,
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -2),
        scale=(1, 1, 1),
    )
    bpy.ops.object.squishy_volumes_add_input_object(object_enum="Collider")

    bpy.ops.scene.squishy_volumes_write_input_to_cache()

    bpy.ops.scene.squishy_volumes_wait_until_finished(
        simulation_uuid=simulation_uuid, timeout_sec=10
    )

    bpy.ops.scene.squishy_volumes_add_output_object(
        object_name="SOLID_PARTICLES - Cube",
        output_type="SOLID_PARTICLES",
        input_name="Cube",
        num_colliders=1,
    )

    bpy.context.scene.frame_end = 100
    bpy.context.scene.render.resolution_percentage = 20

    bpy.context.scene.render.filepath = test_dir
    if bpy.app.version >= (5, 0, 0):
        bpy.context.scene.render.image_settings.media_type = "VIDEO"
    else:
        bpy.context.scene.render.image_settings.file_format = "FFMPEG"
    bpy.context.scene.render.ffmpeg.format = "MPEG4"
    bpy.context.scene.render.ffmpeg.codec = "H264"
    bpy.context.scene.render.ffmpeg.constant_rate_factor = "MEDIUM"
    bpy.context.scene.render.ffmpeg.ffmpeg_preset = "GOOD"
    bpy.context.scene.render.ffmpeg.gopsize = 250
    bpy.context.scene.render.ffmpeg.use_max_b_frames = True
    bpy.context.scene.render.ffmpeg.max_b_frames = 2

    bpy.ops.render.render(animation=True)

    bpy.ops.wm.save_as_mainfile(filepath=f"{test_dir}{test_name}.blend")

    test_uninstall(URL)
