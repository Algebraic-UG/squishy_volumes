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

    bpy.context.preferences.system.use_online_access = True  # ty:ignore[possibly-missing-attribute]

    test_install(URL)

    bpy.ops.scene.squishy_volumes_add_simulation()  # ty:ignore[unresolved-attribute]
    simulation = bpy.context.scene.squishy_volumes_scene.simulations[0]  # ty:ignore[unresolved-attribute]
    simulation.directory = test_dir

    bpy.ops.mesh.primitive_uv_sphere_add(
        radius=1,
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, 3),
        scale=(1, 1, 1),
    )
    bpy.ops.mesh.primitive_plane_add(
        size=6,
        enter_editmode=False,
        align="WORLD",
        location=(0, 0, -2),
        scale=(1, 1, 1),
    )

    cube = bpy.data.objects["Cube"]
    sphere = bpy.data.objects["Sphere"]
    plane = bpy.data.objects["Plane"]

    cube.hide_render = True
    sphere.hide_render = True

    plane.squishy_volumes_object.input_settings.input_type = "Collider"  # ty:ignore[unresolved-attribute]

    cube.select_set(True)
    sphere.select_set(True)
    plane.select_set(True)

    bpy.ops.scene.squishy_volumes_add_input_objects()  # ty:ignore[unresolved-attribute]

    sphere.modifiers["Squishy Volumes Input"]["Socket_8"] = 3
    sphere.modifiers["Squishy Volumes Input"].node_group.interface_update(bpy.context)

    bpy.ops.scene.squishy_volumes_write_input_to_cache(blocking=True)  # ty:ignore[unresolved-attribute]

    bpy.ops.scene.squishy_volumes_wait_until_finished(  # ty:ignore[unresolved-attribute]
        simulation_uuid=simulation.uuid, timeout_sec=10
    )

    bpy.ops.scene.squishy_volumes_add_output_objects(  # ty:ignore[unresolved-attribute]
        uuid=simulation.uuid,
        input_name="Cube",
        called_from_script=True,
    )
    bpy.ops.scene.squishy_volumes_add_output_objects(  # ty:ignore[unresolved-attribute]
        uuid=simulation.uuid,
        input_name="Sphere",
        called_from_script=True,
    )

    bpy.ops.scene.squishy_volumes_add_output_objects(  # ty:ignore[unresolved-attribute]
        uuid=simulation.uuid, output_type="PARTICLES"
    )

    bpy.context.scene.frame_end = 100  # ty:ignore[invalid-assignment]
    bpy.context.scene.render.resolution_percentage = 20  # ty:ignore[possibly-missing-attribute]

    bpy.context.scene.render.filepath = test_dir  # ty:ignore[possibly-missing-attribute]
    if bpy.app.version >= (5, 0, 0):
        bpy.context.scene.render.image_settings.media_type = "VIDEO"  # ty:ignore[possibly-missing-attribute]
    else:
        bpy.context.scene.render.image_settings.file_format = "FFMPEG"  # ty:ignore[possibly-missing-attribute]
    bpy.context.scene.render.ffmpeg.format = "MPEG4"  # ty:ignore[possibly-missing-attribute, invalid-assignment]
    bpy.context.scene.render.ffmpeg.codec = "H264"  # ty:ignore[possibly-missing-attribute, invalid-assignment]
    bpy.context.scene.render.ffmpeg.constant_rate_factor = "MEDIUM"  # ty:ignore[possibly-missing-attribute, invalid-assignment]
    bpy.context.scene.render.ffmpeg.ffmpeg_preset = "GOOD"  # ty:ignore[possibly-missing-attribute, invalid-assignment]
    bpy.context.scene.render.ffmpeg.gopsize = 250  # ty:ignore[possibly-missing-attribute, invalid-assignment]
    bpy.context.scene.render.ffmpeg.use_max_b_frames = True  # ty:ignore[possibly-missing-attribute, invalid-assignment]
    bpy.context.scene.render.ffmpeg.max_b_frames = 2  # ty:ignore[possibly-missing-attribute, invalid-assignment]

    bpy.ops.render.render(animation=True)

    bpy.ops.wm.save_as_mainfile(filepath=f"{test_dir}{test_name}.blend")

    test_uninstall(URL)
