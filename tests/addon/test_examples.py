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

import colorsys
import time
from types import SimpleNamespace
from unittest import mock

import bpy
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Vector

from .support import (
    AddonTestCase,
    addon,
    cache_directory,
    input_objects,
    simulation_handle,
    simulation_objects,
)

BAKE_TIMEOUT_SEC = 300


class ExampleTestCase(AddonTestCase):
    example = ""

    def setUp(self):
        super().setUp()
        self.user_camera = bpy.data.objects.new(
            "User Camera", bpy.data.cameras.new("User Camera")
        )
        bpy.context.scene.collection.objects.link(self.user_camera)
        bpy.context.scene.camera = self.user_camera
        bpy.ops.scene.squishy_volumes_add_example_simulation(
            startup_choice=self.example
        )
        [self.sim_obj] = simulation_objects()
        self.baking_after_setup = addon().api.is_baking(self.sim_obj)
        self.baked_after_setup = addon().api.baked_frames(self.sim_obj)

    def tearDown(self):
        addon().api.pause_bake(self.sim_obj)
        addon().api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        super().tearDown()

    def assert_started_baking(self):
        self.assertTrue(self.baking_after_setup or self.baked_after_setup > 0)


class StagingTests:
    def example_collection(self) -> bpy.types.Collection:
        return bpy.data.collections[self.sim_obj.name]

    def staged_objects(self, object_type: str) -> list[bpy.types.Object]:
        return [
            obj for obj in self.example_collection().objects if obj.type == object_type
        ]

    def test_keeps_everything_in_its_own_collection(self):
        collection = self.example_collection()
        self.assertIn(collection, list(bpy.context.scene.collection.children))
        outputs = [obj for obj in bpy.data.objects if obj.name.endswith(" - Output")]
        for obj in [*input_objects(self.sim_obj), *outputs]:
            self.assertEqual(list(obj.users_collection), [collection], obj.name)
        self.assertEqual(list(bpy.context.scene.collection.objects), [self.user_camera])

    def inputs_of_type(self, input_type: str) -> list[bpy.types.Object]:
        return [
            obj
            for obj in input_objects(self.sim_obj)
            if obj.squishy_volumes.input_type == input_type
        ]

    def outputs_by_input(self) -> dict[bpy.types.Object, bpy.types.Object]:
        return {
            obj: bpy.data.objects[f"{obj.name} - Output"]
            for obj in self.inputs_of_type("Particles")
        }

    def assert_camera_sees(self, objects: list[bpy.types.Object]):
        [camera] = self.staged_objects("CAMERA")
        scene = bpy.context.scene
        for obj in objects:
            for corner in obj.bound_box:
                view = world_to_camera_view(
                    scene, camera, obj.matrix_world @ Vector(corner)
                )
                self.assertTrue(0.0 <= view.x <= 1.0, obj.name)
                self.assertTrue(0.0 <= view.y <= 1.0, obj.name)
                self.assertGreater(view.z, 0.0, obj.name)

    def test_camera_sees_all_particle_inputs(self):
        bpy.context.scene.frame_set(bpy.context.scene.frame_start)

        self.assert_camera_sees(self.inputs_of_type("Particles"))

    def test_inputs_and_outputs_get_palette_materials(self):
        for input_type in ("Particles", "Collider"):
            for obj in self.inputs_of_type(input_type):
                material = obj.material_slots[0].material
                self.assertTrue(
                    material.name.startswith(f"Squishy Volumes {input_type} "),
                    obj.name,
                )
        for obj, output_obj in self.outputs_by_input().items():
            self.assertEqual(
                output_obj.material_slots[0].material,
                obj.material_slots[0].material,
            )

    def test_colors_are_distinct_within_each_input_type(self):
        for input_type in ("Particles", "Collider"):
            colors = [
                tuple(obj.material_slots[0].material.diffuse_color)
                for obj in self.inputs_of_type(input_type)
            ]
            self.assertEqual(len(set(colors)), len(colors), input_type)

    def test_visualization_scale(self):
        for output_obj in self.outputs_by_input().values():
            modifier = output_obj.modifiers["Squishy Volumes Output"]
            identifier = modifier.node_group.interface.items_tree["Scale"].identifier
            scale = getattr(modifier.properties.inputs, identifier).value
            self.assertAlmostEqual(scale, self.visualization_scale, places=6)

    def test_adding_again_reuses_the_materials(self):
        materials = set(bpy.data.materials)

        bpy.ops.scene.squishy_volumes_add_example_simulation(
            startup_choice=self.example
        )

        self.assertEqual(set(bpy.data.materials), materials)
        for sim_obj in simulation_objects():
            if sim_obj != self.sim_obj:
                addon().api.pause_bake(sim_obj)
                addon().api.wait_until_baked(sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)

    def test_adds_a_light(self):
        self.assertEqual(len(self.staged_objects("LIGHT")), 1)

    def test_leaves_the_scene_camera_alone(self):
        self.assertEqual(bpy.context.scene.camera, self.user_camera)


class BoingBlockExampleTest(StagingTests, ExampleTestCase):
    example = "Boing Block"
    visualization_scale = 0.1

    def test_records_the_cube_and_the_plane(self):
        self.assertCountEqual(
            [obj.name for obj in input_objects(self.sim_obj)], ["Cube", "Plane"]
        )
        self.assertTrue(
            (cache_directory(self.sim_obj) / "simulation_input.bin").exists()
        )
        self.assertIn("Cube - Output", bpy.data.objects)

    def test_starts_baking(self):
        self.assert_started_baking()

    def test_rebake_provides_all_requested_frames(self):
        api = addon().api
        deadline = time.monotonic() + BAKE_TIMEOUT_SEC
        while api.baked_frames(self.sim_obj) < 1 and time.monotonic() < deadline:
            time.sleep(0.01)
        api.pause_bake(self.sim_obj)
        api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        self.sim_obj.squishy_volumes.compute_device = "CPU"

        api.bake(self.sim_obj, from_frame=0, bake_frames=3)

        self.assertEqual(
            api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC), 3
        )
        sim_handle = simulation_handle(self.sim_obj)
        self.assertIsNone(sim_handle.last_error)
        self.assertEqual(api.bake_errors(self.sim_obj), [])
        stats = addon().util.read_stats(self.sim_obj.squishy_volumes.directory)
        self.assertEqual(sorted(stats), [0, 1, 2])


class BenchmarkExampleTest(StagingTests, ExampleTestCase):
    example = "Benchmark"
    visualization_scale = 0.05

    def test_camera_moves_down_to_the_landing_colliders(self):
        [camera] = self.staged_objects("CAMERA")
        scene = bpy.context.scene
        heights = {}
        for frame in (scene.frame_start, 30, 60):
            scene.frame_set(frame)
            heights[frame] = camera.matrix_world.translation.z
        self.assertAlmostEqual(heights[scene.frame_start], 12.0, places=4)
        self.assertLess(heights[60], heights[30])
        self.assertLess(heights[30], heights[scene.frame_start])
        self.assertAlmostEqual(heights[60], 4.37588, places=4)

        scene.frame_set(60)

        landing = [
            obj
            for obj in self.inputs_of_type("Collider")
            if obj.name.startswith(("Cone", "Torus"))
        ]
        self.assertEqual(len(landing), 2)
        self.assert_camera_sees(landing)

    def test_uses_eight_particle_and_three_collider_colors(self):
        for input_type, count in (("Particles", 8), ("Collider", 3)):
            materials = {
                obj.material_slots[0].material
                for obj in self.inputs_of_type(input_type)
            }
            self.assertEqual(len(materials), count, input_type)

    def test_records_the_monkeys_and_colliders(self):
        input_types = [
            obj.squishy_volumes.input_type for obj in input_objects(self.sim_obj)
        ]
        self.assertEqual(input_types.count("Particles"), 8)
        self.assertEqual(input_types.count("Collider"), 3)
        self.assertTrue(
            (cache_directory(self.sim_obj) / "simulation_input.bin").exists()
        )

    def test_starts_baking(self):
        self.assert_started_baking()


class ExampleInNewSceneTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.original_scene = bpy.context.scene
        bpy.ops.scene.squishy_volumes_add_example_simulation(
            startup_choice="Boing Block", new_scene=True
        )
        [self.sim_obj] = simulation_objects()

    def tearDown(self):
        addon().api.pause_bake(self.sim_obj)
        addon().api.wait_until_baked(self.sim_obj, timeout_sec=BAKE_TIMEOUT_SEC)
        super().tearDown()

    def test_switches_to_a_scene_holding_the_example(self):
        scene = bpy.context.scene
        self.assertNotEqual(scene, self.original_scene)
        self.assertEqual(scene.name, "Boing Block")
        self.assertIn(
            bpy.data.collections[self.sim_obj.name], list(scene.collection.children)
        )
        self.assertEqual(
            [marker.name for marker in scene.timeline_markers], [self.sim_obj.name] * 2
        )

    def test_leaves_the_original_scene_untouched(self):
        self.assertEqual(list(self.original_scene.collection.all_objects), [])
        self.assertEqual(list(self.original_scene.collection.children), [])
        self.assertEqual(len(self.original_scene.timeline_markers), 0)


class ExampleWithoutDiskSpaceTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.preferences = addon().get_preferences.get_preferences()
        self.checked_disk_space = self.preferences.sanity_check_allowed_disk_space
        self.preferences.sanity_check_allowed_disk_space = True

    def tearDown(self):
        self.preferences.sanity_check_allowed_disk_space = self.checked_disk_space
        super().tearDown()

    def test_setup_finishes_without_baking(self):
        with mock.patch("shutil.disk_usage", return_value=SimpleNamespace(free=0)):
            result = bpy.ops.scene.squishy_volumes_add_example_simulation(
                startup_choice="Boing Block"
            )

        self.assertEqual(result, {"FINISHED"})
        [sim_obj] = simulation_objects()
        self.assertEqual(bpy.context.scene.squishy_volumes.selected_simulation, sim_obj)
        self.assertFalse(addon().api.is_baking(sim_obj))
        self.assertEqual(addon().api.baked_frames(sim_obj), 0)
        self.assertIn("run out of space", simulation_handle(sim_obj).last_error)


class ExampleWithFailedRecordingTest(AddonTestCase):
    def test_setup_finishes_without_recording(self):
        api = addon().api
        failure = api.CaptureRangeError("no frames to record")
        with mock.patch.object(api, "record_input", side_effect=failure):
            result = bpy.ops.scene.squishy_volumes_add_example_simulation(
                startup_choice="Boing Block"
            )

        self.assertEqual(result, {"FINISHED"})
        [sim_obj] = simulation_objects()
        self.assertEqual(bpy.context.scene.squishy_volumes.selected_simulation, sim_obj)
        self.assertCountEqual(
            [obj.name for obj in input_objects(sim_obj)], ["Cube", "Plane"]
        )
        self.assertIsNone(simulation_handle(sim_obj))


class ExampleInUnsavedFileTest(AddonTestCase):
    def test_refuses_before_changing_anything(self):
        bpy.ops.wm.read_homefile(use_empty=True)

        with self.assertRaisesRegex(RuntimeError, "Save the .blend file first"):
            bpy.ops.scene.squishy_volumes_add_example_simulation(
                startup_choice="Boing Block"
            )

        self.assertEqual(list(bpy.data.objects), [])
        self.assertEqual(list(bpy.data.collections), [])
        self.assertEqual(list(bpy.data.meshes), [])
        self.assertEqual(list(bpy.data.cache_files), [])


class PaletteTest(AddonTestCase):
    def palette(self):
        return addon().example.palette

    def saturations(self, colors) -> list[float]:
        return [
            colorsys.rgb_to_hsv(*self.palette().linear_color(color)[:3])[1]
            for color in colors
        ]

    def test_has_eight_particle_and_four_collider_colors(self):
        palette = self.palette()
        self.assertEqual(len(set(palette.PARTICLE_COLORS)), 8)
        self.assertEqual(len(set(palette.COLLIDER_COLORS)), 4)

    def test_colliders_are_calmer_than_particles(self):
        palette = self.palette()
        self.assertLess(
            max(self.saturations(palette.COLLIDER_COLORS)),
            min(self.saturations(palette.PARTICLE_COLORS)),
        )

    def test_materials_are_reused_by_index(self):
        palette = self.palette()
        material = palette.particle_material(2)

        self.assertIs(palette.particle_material(2), material)
        self.assertIsNot(palette.collider_material(2), material)
