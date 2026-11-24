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

import time
import unittest
from pathlib import Path

import bpy

from .support import AddonTestCase, addon, api, video_dir, video_preset

BAKE_TIMEOUT_SEC = 1800


def rendered_videos(directory: Path, prefix: str) -> list[Path]:
    return [path for path in directory.glob(f"{prefix}*.mp4") if path.stat().st_size]


class MultiMaterialScene(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = addon().example.setup_example_simulation(
            bpy.context, "Multi Material"
        )
        self.sim_obj.squishy_volumes.compute_device = "CPU"
        api().record_input(self.sim_obj)
        self.camera = bpy.data.objects[f"{self.sim_obj.name} Camera"]

    def bake(self, frames: int) -> int:
        self.sim_obj.squishy_volumes.bake_frames = frames
        api().bake(self.sim_obj)
        return api().wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def set_up_render(self, preset: str, filepath: str):
        api().set_up_render(self.sim_obj, preset, camera=self.camera, filepath=filepath)


class TinyPreviewRenderTest(MultiMaterialScene):
    def test_renders_a_preview_video(self):
        self.assertEqual(self.bake(3), 3)
        scene = bpy.context.scene
        self.set_up_render(api().PREVIEW, str(self.work_dir / "tiny_"))
        scene.render.resolution_percentage = 5

        bpy.ops.render.render(animation=True)

        self.assertEqual((scene.frame_start, scene.frame_end), (1, 3))
        self.assertEqual(len(rendered_videos(self.work_dir, "tiny_")), 1)


@unittest.skipUnless(video_dir(), "renders only with --video-dir")
class MultiMaterialVideoTest(MultiMaterialScene):
    def test_renders_the_whole_bake(self):
        preset = video_preset()
        frames = api().RENDER_PRESETS[preset].max_frames
        self.assertEqual(self.bake(frames), frames)
        prefix = f"multi_material_{preset.lower()}_"
        self.set_up_render(preset, str(video_dir() / prefix))

        start = time.monotonic()
        bpy.ops.render.render(animation=True)
        print(f"rendered {frames} frames in {time.monotonic() - start:.1f} s")

        self.assertEqual(len(rendered_videos(video_dir(), prefix)), 1)
