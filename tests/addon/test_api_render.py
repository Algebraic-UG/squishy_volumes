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

from .support import AddonTestCase, api


def add_camera(name: str, scene: bpy.types.Scene | None = None) -> bpy.types.Object:
    camera = bpy.data.objects.new(name, bpy.data.cameras.new(name))
    (scene or bpy.context.scene).collection.objects.link(camera)
    return camera


def render_state(scene: bpy.types.Scene) -> tuple:
    render = scene.render
    return (
        scene.camera,
        scene.frame_start,
        scene.frame_end,
        render.engine,
        render.resolution_x,
        render.resolution_y,
        render.resolution_percentage,
        render.filepath,
        render.image_settings.media_type,
        scene.display.shading.color_type,
    )


class RenderTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.scene = bpy.context.scene
        self.sim_obj = api().add_simulation("Jelly")
        api().set_capture_range(self.sim_obj, 5, 5)
        self.sim_obj.squishy_volumes.bake_frames = 60
        self.camera = add_camera("Camera")


class SetUpRenderTest(RenderTestCase):
    def test_preview_is_a_small_quick_video(self):
        api().set_up_render(self.sim_obj, api().PREVIEW, camera=self.camera)

        render = self.scene.render
        self.assertEqual(self.scene.camera, self.camera)
        self.assertEqual(render.engine, "BLENDER_WORKBENCH")
        self.assertEqual((render.resolution_x, render.resolution_y), (1920, 1080))
        self.assertEqual(render.resolution_percentage, 20)
        shading = self.scene.display.shading
        self.assertEqual(shading.light, "STUDIO")
        self.assertEqual(shading.color_type, "MATERIAL")
        self.assertTrue(shading.show_shadows)
        self.assertAlmostEqual(shading.shadow_intensity, 0.5)
        self.assertTrue(shading.show_cavity)
        self.assertEqual(shading.cavity_type, "BOTH")
        self.assertTrue(shading.show_specular_highlight)
        self.assertFalse(shading.show_object_outline)
        self.assertEqual(self.scene.display.render_aa, "16")
        self.assertEqual(render.image_settings.media_type, "VIDEO")
        self.assertEqual(render.ffmpeg.format, "MPEG4")
        self.assertEqual(render.ffmpeg.codec, "H264")
        self.assertEqual(render.filepath, "//renders/Jelly preview ")

    def test_renders_the_baked_frames_from_the_capture_start(self):
        api().set_up_render(self.sim_obj, api().PREVIEW, camera=self.camera)

        self.assertEqual((self.scene.frame_start, self.scene.frame_end), (5, 64))

    def test_preview_is_capped_at_one_hundred_frames(self):
        self.sim_obj.squishy_volumes.bake_frames = 250

        api().set_up_render(self.sim_obj, api().PREVIEW, camera=self.camera)

        self.assertEqual((self.scene.frame_start, self.scene.frame_end), (5, 104))

    def test_showcase_is_a_larger_cycles_video(self):
        self.scene.display.shading.color_type = "OBJECT"
        device = self.scene.cycles.device

        api().set_up_render(self.sim_obj, api().SHOWCASE, camera=self.camera)

        render = self.scene.render
        cycles = self.scene.cycles
        self.assertEqual(render.engine, "CYCLES")
        self.assertEqual((render.resolution_x, render.resolution_y), (1280, 720))
        self.assertEqual(render.resolution_percentage, 100)
        self.assertEqual(render.ffmpeg.constant_rate_factor, "HIGH")
        self.assertLessEqual(self.scene.frame_end - self.scene.frame_start + 1, 240)
        self.assertEqual(cycles.samples, 128)
        self.assertTrue(cycles.use_adaptive_sampling)
        self.assertAlmostEqual(cycles.adaptive_threshold, 0.02)
        self.assertTrue(cycles.use_denoising)
        self.assertEqual(cycles.max_bounces, 4)
        self.assertEqual(cycles.device, device)
        self.assertEqual(self.scene.display.shading.color_type, "OBJECT")

    def test_output_path_can_be_given(self):
        api().set_up_render(
            self.sim_obj, api().PREVIEW, camera=self.camera, filepath="//clips/jelly_"
        )

        self.assertEqual(self.scene.render.filepath, "//clips/jelly_")

    def test_only_changes_the_given_scene(self):
        other = bpy.data.scenes.new("Other")
        other_state = render_state(other)

        api().set_up_render(self.sim_obj, api().PREVIEW, camera=self.camera)

        self.assertEqual(render_state(other), other_state)


class InvalidRenderSetupTest(RenderTestCase):
    def assert_rejected(self, preset, camera):
        before = render_state(self.scene)

        with self.assertRaises(api().InvalidRenderSetup):
            api().set_up_render(self.sim_obj, preset, camera=camera)

        self.assertEqual(render_state(self.scene), before)

    def test_rejects_unknown_presets(self):
        self.assert_rejected("CINEMA", self.camera)

    def test_rejects_a_missing_camera(self):
        self.assert_rejected(api().PREVIEW, None)

    def test_rejects_objects_that_are_not_cameras(self):
        empty = bpy.data.objects.new("Empty", None)
        self.scene.collection.objects.link(empty)

        self.assert_rejected(api().PREVIEW, empty)

    def test_rejects_cameras_from_other_scenes(self):
        elsewhere = add_camera("Elsewhere", bpy.data.scenes.new("Other"))

        self.assert_rejected(api().PREVIEW, elsewhere)

    def test_errors_are_squishy_volumes_errors(self):
        self.assertTrue(issubclass(api().InvalidRenderSetup, api().SquishyVolumesError))
