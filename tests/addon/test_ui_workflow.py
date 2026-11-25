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

from .support import AddonTestCase, api, marker_frames, simulation_objects
from .test_failures_leave_no_changes import new_input_item

BAKE_TIMEOUT_SEC = 300


def add_primitive(add_operator, name: str, **kwargs) -> bpy.types.Object:
    add_operator(**kwargs)
    obj = bpy.context.active_object
    obj.name = name
    return obj


def add_camera() -> bpy.types.Object:
    camera = bpy.data.objects.new("Camera", bpy.data.cameras.new("Camera"))
    bpy.context.scene.collection.objects.link(camera)
    camera.location = (0, -12, 2)
    camera.rotation_euler = (1.45, 0, 0)
    return camera


def uuid_of(sim_obj: bpy.types.Object) -> str:
    return sim_obj.squishy_volumes.uuid


def add_simulation(name: str) -> bpy.types.Object:
    bpy.ops.scene.squishy_volumes_add_simulation(sim_name=name)
    sim_obj = bpy.context.scene.squishy_volumes.selected_simulation
    sim_obj.squishy_volumes.compute_device = "CPU"
    return sim_obj


def select_simulation(sim_obj: bpy.types.Object):
    bpy.context.scene.squishy_volumes.selected_simulation = sim_obj


def add_inputs(sim_obj: bpy.types.Object, *objects_and_types):
    bpy.ops.object.select_all(action="DESELECT")
    inputs = []
    for obj, input_type in objects_and_types:
        obj.select_set(True)
        inputs.append({**new_input_item(obj.name), "input_type": input_type})
    result = bpy.ops.scene.squishy_volumes_add_input_objects(
        uuid=uuid_of(sim_obj), inputs=inputs
    )
    assert result == {"FINISHED"}, result


def record_and_bake(sim_obj: bpy.types.Object, frames: int) -> int:
    sim_obj.squishy_volumes.bake_frames = frames
    bpy.ops.scene.squishy_volumes_record_input_to_cache(
        uuid=uuid_of(sim_obj), start_baking=True
    )
    return api().wait_until_baked(sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)


def bake_from_latest(sim_obj: bpy.types.Object, frames: int) -> int:
    sim_obj.squishy_volumes.bake_frames = frames
    bpy.ops.scene.squishy_volumes_bake_start_from_latest(uuid=uuid_of(sim_obj))
    return api().wait_until_baked(sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)


def rebake_from(sim_obj: bpy.types.Object, scene_frame: int) -> int:
    bpy.context.scene.frame_set(scene_frame)
    bpy.ops.scene.squishy_volumes_bake_start_from_loaded(uuid=uuid_of(sim_obj))
    return api().wait_until_baked(sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)


def shown_points(obj: bpy.types.Object, scene_frame: int) -> int:
    bpy.context.scene.frame_set(scene_frame)
    evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
    return len(evaluated.data.points)


def modifier_names(obj: bpy.types.Object) -> list[str]:
    return [modifier.name for modifier in obj.modifiers]


def set_up_preview_render(sim_obj: bpy.types.Object, camera: bpy.types.Object):
    result = bpy.ops.scene.squishy_volumes_set_up_render(
        simulation=sim_obj.name, preset="PREVIEW", camera=camera.name
    )
    assert result == {"FINISHED"}, result


def rendered_videos(directory: Path) -> list[Path]:
    return [path for path in directory.glob("*.mp4") if path.stat().st_size]


class UiWorkflowTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.camera = add_camera()
        self.cube = add_primitive(bpy.ops.mesh.primitive_cube_add, "Jelly Cube")
        self.floor = add_primitive(
            bpy.ops.mesh.primitive_plane_add, "Floor", size=10, location=(0, 0, -2)
        )

    def assert_render_set_up(self, sim_obj: bpy.types.Object, frames: int):
        scene = bpy.context.scene
        self.assertEqual(scene.camera, self.camera)
        start = sim_obj.squishy_volumes.capture_start_frame
        self.assertEqual(
            (scene.frame_start, scene.frame_end), (start, start + frames - 1)
        )
        self.assertEqual(scene.render.engine, "BLENDER_WORKBENCH")
        self.assertEqual(scene.render.image_settings.media_type, "VIDEO")
        self.assertEqual(scene.render.filepath, f"//renders/{sim_obj.name} preview ")


class FirstSimulationTest(UiWorkflowTestCase):
    def test_from_empty_scene_to_rendered_preview(self):
        sim_obj = add_simulation("Jelly")
        self.assertEqual(simulation_objects(), [sim_obj])
        self.assertEqual(marker_frames(sim_obj), [1, 1])

        add_inputs(sim_obj, (self.cube, api().PARTICLES), (self.floor, api().COLLIDER))
        output = bpy.data.objects["Jelly Cube - Output"]
        self.assertCountEqual(api().input_objects(sim_obj), [self.cube, self.floor])
        self.assertEqual(modifier_names(self.cube), ["Squishy Volumes Input"])
        self.assertEqual(modifier_names(self.floor), ["Squishy Volumes Input"])
        self.assertEqual(
            modifier_names(output),
            ["Squishy Volumes Load Output", "Squishy Volumes Output"],
        )
        self.assertNotIn("Floor - Output", bpy.data.objects)

        self.assertEqual(record_and_bake(sim_obj, 3), 3)
        self.assertFalse(bpy.ops.scene.squishy_volumes_bake_start_from_latest.poll())
        for scene_frame in (1, 2, 3):
            self.assertGreater(shown_points(output, scene_frame), 0)

        set_up_preview_render(sim_obj, self.camera)
        self.assert_render_set_up(sim_obj, 3)

        bpy.context.scene.render.resolution_percentage = 5
        bpy.context.scene.frame_end = bpy.context.scene.frame_start
        bpy.ops.render.render(animation=True)
        self.assertEqual(len(rendered_videos(self.work_dir / "renders")), 1)


class RebakeTest(UiWorkflowTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = add_simulation("Jelly")
        add_inputs(
            self.sim_obj, (self.cube, api().PARTICLES), (self.floor, api().COLLIDER)
        )
        self.output = bpy.data.objects["Jelly Cube - Output"]
        self.assertEqual(record_and_bake(self.sim_obj, 2), 2)

    def test_continue_then_rebake_from_an_earlier_frame(self):
        self.assertTrue(bpy.ops.scene.squishy_volumes_bake_start_from_loaded.poll())
        bpy.context.scene.frame_set(2)
        self.assertFalse(bpy.ops.scene.squishy_volumes_bake_start_from_loaded.poll())

        self.sim_obj.squishy_volumes.bake_frames = 4
        self.assertTrue(bpy.ops.scene.squishy_volumes_bake_start_from_latest.poll())
        self.assertEqual(bake_from_latest(self.sim_obj, 4), 4)
        self.assertFalse(bpy.ops.scene.squishy_volumes_bake_start_from_latest.poll())

        self.assertEqual(rebake_from(self.sim_obj, 2), 4)
        self.assertEqual(api().bake_errors(self.sim_obj), [])
        for scene_frame in (1, 2, 3, 4):
            self.assertGreater(shown_points(self.output, scene_frame), 0)

        set_up_preview_render(self.sim_obj, self.camera)
        self.assert_render_set_up(self.sim_obj, 4)

    def test_rerecording_after_moving_the_markers_starts_over(self):
        for marker, frame in zip(
            bpy.context.scene.timeline_markers, (5, 6), strict=True
        ):
            marker.frame = frame

        self.assertEqual(record_and_bake(self.sim_obj, 2), 2)

        self.assertEqual(self.sim_obj.squishy_volumes.capture_start_frame, 5)
        self.assertGreater(shown_points(self.output, 6), 0)
        set_up_preview_render(self.sim_obj, self.camera)
        self.assert_render_set_up(self.sim_obj, 2)


class RemoveAlongTheWayTest(UiWorkflowTestCase):
    def test_removed_input_is_not_simulated(self):
        sphere = add_primitive(
            bpy.ops.mesh.primitive_ico_sphere_add, "Ball", location=(3, 0, 0)
        )
        sim_obj = add_simulation("Jelly")
        add_inputs(
            sim_obj,
            (self.cube, api().PARTICLES),
            (sphere, api().PARTICLES),
            (self.floor, api().COLLIDER),
        )

        result = bpy.ops.object.squishy_volumes_remove_input_object(name=sphere.name)

        self.assertEqual(result, {"FINISHED"})
        self.assertCountEqual(api().input_objects(sim_obj), [self.cube, self.floor])
        self.assertEqual(record_and_bake(sim_obj, 2), 2)
        self.assertGreater(shown_points(bpy.data.objects["Jelly Cube - Output"], 2), 0)
        self.assertEqual(shown_points(bpy.data.objects["Ball - Output"], 2), 0)

    def test_render_the_remaining_simulation(self):
        doomed = add_simulation("Doomed")
        add_inputs(doomed, (self.cube, api().PARTICLES))
        self.assertEqual(record_and_bake(doomed, 2), 2)
        kept = add_simulation("Kept")
        add_inputs(kept, (self.floor, api().COLLIDER))
        ball = add_primitive(bpy.ops.mesh.primitive_ico_sphere_add, "Ball")
        add_inputs(kept, (ball, api().PARTICLES))

        result = bpy.ops.scene.squishy_volumes_remove_simulation(uuid=uuid_of(doomed))

        self.assertEqual(result, {"FINISHED"})
        self.assertEqual(simulation_objects(), [kept])
        self.assertEqual(self.cube.squishy_volumes.uuid, "")
        self.assertEqual(
            {marker.name for marker in bpy.context.scene.timeline_markers}, {"Kept"}
        )
        select_simulation(kept)
        self.assertEqual(record_and_bake(kept, 3), 3)
        self.assertGreater(shown_points(bpy.data.objects["Ball - Output"], 3), 0)

        set_up_preview_render(kept, self.camera)
        self.assert_render_set_up(kept, 3)
