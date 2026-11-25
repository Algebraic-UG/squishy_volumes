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


from contextlib import contextmanager
from types import SimpleNamespace
from unittest import mock

import addon_utils
import bpy

from .support import (
    AddonTestCase,
    add_mesh_object,
    addon,
    api,
    cache_directory,
    make_input,
    simulation_handle,
)


def data_state() -> dict:
    return {
        "objects": {
            obj.name: (
                obj.squishy_volumes.uuid,
                tuple(modifier.name for modifier in obj.modifiers),
                tuple(collection.name for collection in obj.users_collection),
                obj.animation_data is not None and len(obj.animation_data.drivers),
                tuple(
                    attribute.name
                    for attribute in obj.squishy_volumes.alembic_attributes
                ),
            )
            for obj in bpy.data.objects
        },
        "collections": sorted(collection.name for collection in bpy.data.collections),
        "cache_files": sorted(cache_file.name for cache_file in bpy.data.cache_files),
        "node_groups": sorted(group.name for group in bpy.data.node_groups),
        "meshes": sorted(mesh.name for mesh in bpy.data.meshes),
        "markers": sorted(
            (marker.name, marker.frame) for marker in bpy.context.scene.timeline_markers
        ),
    }


def add_driven_modifier(obj: bpy.types.Object):
    group = bpy.data.node_groups.new(f"{obj.name} Generator", "GeometryNodeTree")
    group.interface.new_socket(
        "Grid Node Size", in_out="INPUT", socket_type="NodeSocketFloat"
    )
    obj.modifiers.new("Squishy Volumes Input", type="NODES").node_group = group


def link_from_library(work_dir, name: str, uuid: str = "") -> bpy.types.Object:
    collection = bpy.data.collections.new(f"{name} Library")
    obj = bpy.data.objects.new(name, bpy.data.meshes.new(name))
    collection.objects.link(obj)
    obj.squishy_volumes.uuid = uuid
    add_driven_modifier(obj)
    library_path = str(work_dir / f"{name}.blend")
    bpy.data.libraries.write(library_path, {collection})
    group = obj.modifiers[0].node_group
    mesh = obj.data
    bpy.data.objects.remove(obj)
    bpy.data.meshes.remove(mesh)
    bpy.data.node_groups.remove(group)
    bpy.data.collections.remove(collection)

    with bpy.data.libraries.load(library_path, link=True) as (_, linked):
        linked.collections = [f"{name} Library"]
    [linked_collection] = linked.collections
    bpy.context.scene.collection.children.link(linked_collection)
    [linked_obj] = linked_collection.objects
    return linked_obj


@contextmanager
def shown_warnings():
    operator_base = addon().operators.operator_base
    with mock.patch.object(
        operator_base, "show_message", wraps=operator_base.show_message
    ) as show_message:
        yield show_message


def warning_messages(show_message) -> list[str]:
    return [message for (_uuid, message), _ in show_message.call_args_list]


def new_input_item(name: str) -> dict:
    return {
        "name": name,
        "obj_name": name,
        "problem": "",
        "input_type": api().PARTICLES,
        "add_default_generation": True,
        "add_default_visualization": True,
        "add_output_object": True,
        "alembic_attributes": [],
        "selected_alembic_attribute": 0,
    }


class AddSimulationWithFailingCacheFileTest(AddonTestCase):
    def failing_cache_file_open(self):
        cachefile = SimpleNamespace(open=lambda **_: {"CANCELLED"})
        return mock.patch.object(bpy.ops, "cachefile", cachefile, create=True)

    def test_api_raises_before_changing_anything(self):
        before = data_state()

        with self.failing_cache_file_open(), self.assertRaises(RuntimeError):
            api().add_simulation("Jelly")

        self.assertEqual(data_state(), before)

    def test_operator_refuses_before_changing_anything(self):
        before = data_state()

        with self.failing_cache_file_open(), self.assertRaises(RuntimeError):
            bpy.ops.scene.squishy_volumes_add_simulation(sim_name="Jelly")

        self.assertEqual(data_state(), before)
        self.assertIsNone(bpy.context.scene.squishy_volumes.selected_simulation)


class CloneSimulationWithLinkedInputTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.source = api().add_simulation("Source")
        make_input(add_mesh_object("Ball"), self.source)
        link_from_library(self.work_dir, "Rock", self.source.squishy_volumes.uuid)

    def test_api_raises_before_changing_anything(self):
        before = data_state()

        with self.assertRaisesRegex(api().InvalidInput, "Rock"):
            api().clone_simulation(self.source)

        self.assertEqual(data_state(), before)

    def test_operator_refuses_before_changing_anything(self):
        before = data_state()

        with self.assertRaisesRegex(RuntimeError, "linked collection"):
            bpy.ops.scene.squishy_volumes_clone_simulation(
                uuid=self.source.squishy_volumes.uuid
            )

        self.assertEqual(data_state(), before)


class RemoveSimulationWithLinkedInputTest(AddonTestCase):
    def test_refuses_before_changing_anything(self):
        sim_obj = api().add_simulation("Doomed")
        make_input(add_mesh_object("Ball"), sim_obj)
        link_from_library(self.work_dir, "Rock", sim_obj.squishy_volumes.uuid)
        before = data_state()

        with self.assertRaisesRegex(RuntimeError, "Rock is linked"):
            bpy.ops.scene.squishy_volumes_remove_simulation(
                uuid=sim_obj.squishy_volumes.uuid
            )

        self.assertEqual(data_state(), before)


class AddInputTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.ball = add_mesh_object("Ball")
        self.ball.select_set(True)

    def add_input_objects(self, *names: str) -> set[str]:
        return bpy.ops.scene.squishy_volumes_add_input_objects(
            uuid=self.sim_obj.squishy_volumes.uuid,
            inputs=[new_input_item(name) for name in names],
        )

    def test_missing_object_refuses_before_changing_anything(self):
        before = data_state()

        with self.assertRaisesRegex(RuntimeError, "no object named 'Missing'"):
            self.add_input_objects("Ball", "Missing")

        self.assertEqual(data_state(), before)

    def test_api_refuses_a_linked_object_before_changing_anything(self):
        rock = link_from_library(self.work_dir, "Rock")
        before = data_state()

        with self.assertRaisesRegex(api().InvalidInput, "Rock is linked"):
            api().add_input(self.sim_obj, rock)

        self.assertEqual(data_state(), before)

    def test_operator_skips_a_linked_object(self):
        rock = link_from_library(self.work_dir, "Rock")

        result = self.add_input_objects("Ball", "Rock")

        self.assertEqual(result, {"FINISHED"})
        self.assertEqual(api().input_objects(self.sim_obj), [self.ball])
        self.assertEqual(rock.squishy_volumes.uuid, "")


class ToolsOnLinkedObjectTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.ball = add_mesh_object("Ball")
        api().add_input(self.sim_obj, self.ball, output_object=False)
        self.rock = link_from_library(
            self.work_dir, "Rock", self.sim_obj.squishy_volumes.uuid
        )

    def test_add_goals_refuses_before_changing_anything(self):
        before = data_state()

        with self.assertRaisesRegex(api().InvalidInput, "Rock is linked"):
            api().add_goals(self.rock)

        self.assertEqual(data_state(), before)

    def test_restrict_view_refuses_before_changing_anything(self):
        before = data_state()

        with self.assertRaisesRegex(api().InvalidInput, "Rock is linked"):
            api().restrict_view(self.rock, fit_vertices=False)

        self.assertEqual(data_state(), before)

    def test_add_drivers_refuses_before_changing_anything(self):
        before = data_state()

        with self.assertRaisesRegex(api().InvalidInput, "Rock is linked"):
            api().add_drivers(self.rock)

        self.assertEqual(data_state(), before)

    def test_add_drivers_operator_skips_the_linked_object(self):
        self.ball.animation_data_clear()
        self.ball.select_set(True)
        self.rock.select_set(True)

        result = bpy.ops.object.squishy_volumes_add_drivers()

        self.assertEqual(result, {"FINISHED"})
        self.assertGreater(len(self.ball.animation_data.drivers), 0)
        self.assertIsNone(self.rock.animation_data)

    def test_copy_alembic_attributes_skips_the_linked_object(self):
        source = add_mesh_object("Source")
        api().add_input(self.sim_obj, source, output_object=False)
        source.squishy_volumes.alembic_attributes.remove(0)

        updated = api().copy_alembic_attributes(source, [self.ball, self.rock])

        self.assertEqual(updated, [self.ball])
        self.assertEqual(
            len(self.ball.squishy_volumes.alembic_attributes),
            len(source.squishy_volumes.alembic_attributes),
        )
        self.assertEqual(len(self.rock.squishy_volumes.alembic_attributes), 0)


class ReloadAllWithBrokenCacheTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.recorded = self.recorded_simulation("B Recorded")

    def recorded_simulation(self, name: str) -> bpy.types.Object:
        sim_obj = api().add_simulation(name)
        bpy.ops.mesh.primitive_cube_add()
        api().add_input(sim_obj, bpy.context.active_object, output_object=False)
        api().record_input(sim_obj)
        simulation_handle(sim_obj).drop()
        return sim_obj

    def broken_simulation(self, name: str) -> bpy.types.Object:
        sim_obj = api().add_simulation(name)
        directory = cache_directory(sim_obj)
        directory.mkdir(parents=True)
        (directory / "simulation_input.bin").write_bytes(b"not a simulation input")
        return sim_obj

    def tearDown(self):
        for sim_obj in api().simulations():
            sim_handle = simulation_handle(sim_obj)
            if sim_handle is not None:
                sim_handle.drop()
        super().tearDown()

    def test_failure_after_a_reload_finishes_with_a_warning(self):
        broken = self.broken_simulation("C Broken")

        with shown_warnings() as show_message:
            result = bpy.ops.scene.squishy_volumes_reload_all()

        self.assertEqual(result, {"FINISHED"})
        self.assertIsNotNone(simulation_handle(self.recorded))
        self.assertIsNone(simulation_handle(broken))
        [warning] = warning_messages(show_message)
        self.assertIn("C Broken", warning)

    def test_failure_before_any_reload_refuses(self):
        broken = self.broken_simulation("A Broken")

        with self.assertRaises(RuntimeError):
            bpy.ops.scene.squishy_volumes_reload_all()

        self.assertIsNone(simulation_handle(broken))
        self.assertIsNone(simulation_handle(self.recorded))


class RecordAndBakeWithoutDiskSpaceTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.preferences = addon().get_preferences.get_preferences()
        self.checked_disk_space = self.preferences.sanity_check_allowed_disk_space
        self.preferences.sanity_check_allowed_disk_space = True

    def tearDown(self):
        self.preferences.sanity_check_allowed_disk_space = self.checked_disk_space
        super().tearDown()

    def test_recording_finishes_with_a_warning(self):
        sim_obj = api().add_simulation("Jelly")
        bpy.ops.mesh.primitive_cube_add()
        api().add_input(sim_obj, bpy.context.active_object, output_object=False)

        with (
            mock.patch("shutil.disk_usage", return_value=SimpleNamespace(free=0)),
            shown_warnings() as show_message,
        ):
            result = bpy.ops.scene.squishy_volumes_record_input_to_cache(
                uuid=sim_obj.squishy_volumes.uuid, start_baking=True
            )

        self.assertEqual(result, {"FINISHED"})
        self.assertIsNotNone(simulation_handle(sim_obj))
        self.assertFalse(api().is_baking(sim_obj))
        [warning] = warning_messages(show_message)
        self.assertIn("not baking", warning)
        self.assertIn("run out of space", simulation_handle(sim_obj).last_error)


class SetUpRenderWithoutCyclesTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        camera = bpy.data.objects.new("Camera", bpy.data.cameras.new("Camera"))
        bpy.context.scene.collection.objects.link(camera)
        addon_utils.disable("cycles")
        self.addCleanup(addon_utils.enable, "cycles")

    def render_state(self) -> tuple:
        scene = bpy.context.scene
        return (
            scene.camera,
            scene.frame_start,
            scene.frame_end,
            scene.render.engine,
            scene.render.resolution_percentage,
            scene.render.filepath,
        )

    def test_refuses_before_changing_anything(self):
        before = self.render_state()

        with self.assertRaisesRegex(RuntimeError, "Cycles"):
            bpy.ops.scene.squishy_volumes_set_up_render(
                simulation="Jelly", preset=api().SHOWCASE, camera="Camera"
            )

        self.assertEqual(self.render_state(), before)
