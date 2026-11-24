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

from dataclasses import dataclass

import bpy

from ..errors import InvalidRenderSetup
from .simulation import scene_or_current

PREVIEW = "PREVIEW"
SHOWCASE = "SHOWCASE"


@dataclass(frozen=True)
class RenderPreset:
    engine: str
    resolution: tuple[int, int]
    resolution_percentage: int
    max_frames: int
    video_quality: str


RENDER_PRESETS = {
    PREVIEW: RenderPreset(
        engine="BLENDER_WORKBENCH",
        resolution=(1920, 1080),
        resolution_percentage=20,
        max_frames=100,
        video_quality="MEDIUM",
    ),
    SHOWCASE: RenderPreset(
        engine="CYCLES",
        resolution=(1280, 720),
        resolution_percentage=100,
        max_frames=240,
        video_quality="HIGH",
    ),
}


def render_frames(sim_obj: bpy.types.Object, preset: str) -> tuple[int, int]:
    sim_props = sim_obj.squishy_volumes
    frames = min(sim_props.bake_frames, _preset(preset).max_frames)
    start = sim_props.capture_start_frame
    return start, start + frames - 1


def set_up_render(
    sim_obj: bpy.types.Object,
    preset: str,
    *,
    camera: bpy.types.Object | None,
    scene: bpy.types.Scene | None = None,
    filepath: str | None = None,
):
    scene = scene_or_current(scene)
    settings = _preset(preset)
    _check_camera(camera, scene)

    scene.camera = camera
    scene.frame_start, scene.frame_end = render_frames(sim_obj, preset)
    render = scene.render
    render.engine = settings.engine
    render.resolution_x, render.resolution_y = settings.resolution
    render.resolution_percentage = settings.resolution_percentage
    render.filepath = filepath or f"//renders/{sim_obj.name} {preset.lower()} "
    render.image_settings.media_type = "VIDEO"
    render.ffmpeg.format = "MPEG4"
    render.ffmpeg.codec = "H264"
    render.ffmpeg.constant_rate_factor = settings.video_quality
    render.ffmpeg.audio_codec = "NONE"
    ENGINE_SETTINGS[settings.engine](scene)


def _set_up_workbench(scene: bpy.types.Scene):
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "MATERIAL"
    shading.show_shadows = True
    shading.shadow_intensity = 0.5
    shading.show_cavity = True
    shading.cavity_type = "BOTH"
    shading.show_specular_highlight = True
    shading.show_object_outline = False
    scene.display.render_aa = "16"


def _set_up_cycles(scene: bpy.types.Scene):
    cycles = scene.cycles
    cycles.samples = 128
    cycles.use_adaptive_sampling = True
    cycles.adaptive_threshold = 0.02
    cycles.use_denoising = True
    cycles.max_bounces = 4
    cycles.diffuse_bounces = 2
    cycles.glossy_bounces = 2
    cycles.transmission_bounces = 2
    cycles.volume_bounces = 0
    cycles.transparent_max_bounces = 4
    cycles.caustics_reflective = False
    cycles.caustics_refractive = False


ENGINE_SETTINGS = {
    "BLENDER_WORKBENCH": _set_up_workbench,
    "CYCLES": _set_up_cycles,
}


def _preset(preset: str) -> RenderPreset:
    if preset not in RENDER_PRESETS:
        raise InvalidRenderSetup(
            f"Unknown render preset '{preset}', choose one of {sorted(RENDER_PRESETS)}"
        )
    return RENDER_PRESETS[preset]


def _check_camera(camera: bpy.types.Object | None, scene: bpy.types.Scene):
    if camera is None:
        raise InvalidRenderSetup("Choose a camera to render with")
    if camera.type != "CAMERA":
        raise InvalidRenderSetup(f"{camera.name} is not a camera")
    if camera.name not in scene.objects:
        raise InvalidRenderSetup(f"{camera.name} is not in the scene {scene.name}")
