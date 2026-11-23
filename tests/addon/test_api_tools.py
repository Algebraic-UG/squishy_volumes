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
from mathutils import Matrix, Vector

from .support import AddonTestCase, add_mesh_object, api
from .test_api_bake import BakeTestCase


def add_cube(location=(0.0, 0.0, 0.0)) -> bpy.types.Object:
    bpy.ops.mesh.primitive_cube_add(size=2, location=location)
    return bpy.context.active_object


def node_modifier(obj: bpy.types.Object, name: str) -> bpy.types.Modifier:
    modifier = obj.modifiers[name]
    assert modifier.type == "NODES"
    return modifier


def modifier_input(modifier: bpy.types.Modifier, socket_name: str):
    identifier = modifier.node_group.interface.items_tree[socket_name].identifier
    return getattr(modifier.properties.inputs, identifier).value


def drivers_of(obj: bpy.types.Object, modifier: bpy.types.Modifier):
    if obj.animation_data is None:
        return []
    return [
        fcurve.driver
        for fcurve in obj.animation_data.drivers
        if f'modifiers["{modifier.name}"]' in fcurve.data_path
    ]


def assert_matrices_close(test, actual: Matrix, expected: Matrix):
    for actual_row, expected_row in zip(actual, expected, strict=True):
        for actual_value, expected_value in zip(actual_row, expected_row, strict=True):
            test.assertAlmostEqual(actual_value, expected_value, places=5)


class ToolTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.ball = add_cube(location=(1.0, 2.0, 3.0))
        api().add_input(self.sim_obj, self.ball, output_object=False)


class AddGoalsTest(ToolTestCase):
    def test_adds_choose_and_move_objects_driving_the_goal_modifier(self):
        choose, move = api().add_goals(self.ball)

        self.assertEqual(choose.name, "Cube - Choose")
        self.assertEqual(choose.type, "MESH")
        self.assertEqual(len(choose.data.vertices), 42)
        self.assertEqual(move.type, "EMPTY")
        self.assertEqual(move.parent, choose)
        modifier = node_modifier(self.ball, "Squishy Volumes Goals")
        self.assertEqual(modifier.node_group.name, "Squishy Volumes Set Goals")
        self.assertEqual(modifier_input(modifier, "Choose"), choose)
        self.assertEqual(modifier_input(modifier, "Move"), move)

    def test_places_the_objects_next_to_the_input(self):
        choose, move = api().add_goals(self.ball)

        self.assertEqual(tuple(choose.location), (1.0, 2.0, 3.0))
        for obj in (choose, move):
            self.assertEqual(
                list(obj.users_collection), list(self.ball.users_collection)
            )

    def test_location_can_be_given(self):
        choose, _move = api().add_goals(self.ball, location=Vector((5.0, 0.0, 0.0)))

        self.assertEqual(tuple(choose.location), (5.0, 0.0, 0.0))

    def test_rejects_objects_that_are_not_meshes(self):
        empty = bpy.data.objects.new("Empty", None)
        bpy.context.scene.collection.objects.link(empty)
        objects_before = set(bpy.data.objects)

        with self.assertRaises(api().InvalidInput):
            api().add_goals(empty)

        self.assertEqual(set(bpy.data.objects), objects_before)


class RestrictViewTest(ToolTestCase):
    def test_fits_a_box_around_the_vertices(self):
        empty = api().restrict_view(self.ball)

        self.assertEqual(empty.name, "Cube - View")
        self.assertEqual(empty.empty_display_type, "CUBE")
        assert_matrices_close(
            self,
            empty.matrix_world,
            Matrix.LocRotScale((1.0, 2.0, 3.0), None, (1.02, 1.02, 1.02)),
        )
        self.assertEqual(list(empty.users_collection), list(self.ball.users_collection))

    def test_restricts_before_every_other_modifier(self):
        empty = api().restrict_view(self.ball)

        first = self.ball.modifiers[0]
        self.assertEqual(first.name, "Squishy Volumes Restrict View")
        self.assertEqual(modifier_input(first, "Object"), empty)

    def test_name_and_fitting_are_optional(self):
        empty = api().restrict_view(self.ball, name="Window", fit_vertices=False)

        self.assertEqual(empty.name, "Window")
        assert_matrices_close(self, empty.matrix_world, Matrix.Identity(4))

    def test_rejects_meshes_without_vertices_to_fit(self):
        hollow = add_mesh_object("Hollow")
        objects_before = set(bpy.data.objects)

        with self.assertRaises(api().InvalidInput):
            api().restrict_view(hollow)

        self.assertEqual(set(bpy.data.objects), objects_before)
        self.assertEqual(len(hollow.modifiers), 0)

    def test_rejects_objects_without_geometry(self):
        with self.assertRaises(api().InvalidInput):
            api().restrict_view(self.sim_obj)


def evaluated_points(obj: bpy.types.Object) -> list[Vector]:
    evaluated = obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
    return [obj.matrix_world @ point.co for point in evaluated.data.points]


class OutputRestrictViewTest(BakeTestCase):
    def setUp(self):
        super().setUp()
        self.output = bpy.data.objects["Cube - Output"]
        self.output.modifiers["Squishy Volumes Output"].show_viewport = False

    def test_restricts_the_loaded_output(self):
        self.bake_and_wait(bake_frames=1)
        points = evaluated_points(self.output)
        self.assertGreater(len(points), 0)

        empty = api().restrict_view(self.output)

        self.assertEqual(
            [modifier.name for modifier in self.output.modifiers],
            [
                "Squishy Volumes Load Output",
                "Squishy Volumes Restrict View",
                "Squishy Volumes Output",
            ],
        )
        self.assertFalse(self.output.modifiers["Squishy Volumes Output"].show_viewport)
        self.assertEqual(len(evaluated_points(self.output)), len(points))
        to_view = empty.matrix_world.inverted()
        for point in points:
            self.assertLessEqual(max(abs(c) for c in to_view @ point), 1.0)

        empty.scale *= 0.5
        self.assertLess(len(evaluated_points(self.output)), len(points))

    def test_fitting_ignores_the_visualization(self):
        self.output.modifiers["Squishy Volumes Output"].show_viewport = True
        self.bake_and_wait(bake_frames=1)

        empty = api().restrict_view(self.output)

        self.assertTrue(self.output.modifiers["Squishy Volumes Output"].show_viewport)
        self.assertLess(max(empty.scale), 1.1)

    def test_rejects_an_output_without_baked_points(self):
        objects_before = set(bpy.data.objects)

        with self.assertRaises(api().InvalidInput):
            api().restrict_view(self.output)

        self.assertEqual(set(bpy.data.objects), objects_before)
        self.assertEqual(len(self.output.modifiers), 2)


class AddDriversTest(ToolTestCase):
    def test_drives_the_generator_from_the_simulation(self):
        modifier = node_modifier(self.ball, "Squishy Volumes Input")
        for driver in drivers_of(self.ball, modifier):
            driver.variables.remove(driver.variables[0])

        self.assertEqual(api().add_drivers(self.ball), [modifier])

        self.sim_obj.squishy_volumes.grid_node_size = 0.37
        bpy.context.view_layer.update()
        self.assertAlmostEqual(modifier_input(modifier, "Grid Node Size"), 0.37, 5)

    def test_adding_again_keeps_one_variable_per_driver(self):
        modifier = node_modifier(self.ball, "Squishy Volumes Input")

        api().add_drivers(self.ball)
        api().add_drivers(self.ball)

        drivers = drivers_of(self.ball, modifier)
        self.assertTrue(drivers)
        for driver in drivers:
            self.assertEqual(len(driver.variables), 1)
            self.assertEqual(driver.variables[0].targets[0].id, self.sim_obj)

    def test_ignores_modifiers_without_the_prefix(self):
        generator = node_modifier(self.ball, "Squishy Volumes Input")
        other = self.ball.modifiers.new("Other", type="NODES")
        other.node_group = generator.node_group

        self.assertEqual(api().add_drivers(self.ball), [generator])
        self.assertEqual(drivers_of(self.ball, other), [])

    def test_rejects_objects_that_are_not_inputs(self):
        loose = add_cube()

        with self.assertRaises(api().InvalidInput):
            api().add_drivers(loose)


class ToolOperatorTest(ToolTestCase):
    def test_add_drivers_skips_objects_that_are_not_inputs(self):
        loose = add_cube()
        loose.select_set(True)
        self.ball.select_set(True)

        self.assertEqual(bpy.ops.object.squishy_volumes_add_drivers(), {"FINISHED"})

    def test_add_goals_selects_the_choose_object(self):
        scene = bpy.context.scene
        scene.squishy_volumes.selected_input_object = list(bpy.data.objects).index(
            self.ball
        )

        bpy.ops.object.squishy_volumes_input_object_add_goals()

        choose = bpy.data.objects["Cube - Choose"]
        self.assertEqual(bpy.context.view_layer.objects.active, choose)
        self.assertEqual(list(bpy.context.selected_objects), [choose])

    def test_restrict_view_uses_the_active_object(self):
        bpy.context.view_layer.objects.active = self.ball
        self.ball.select_set(True)

        bpy.ops.object.squishy_volumes_restrict_view(empty_name="Window")

        self.assertEqual(
            modifier_input(self.ball.modifiers[0], "Object"),
            bpy.data.objects["Window"],
        )
