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

from .support import AddonTestCase, add_mesh_object, api

PARTICLE_ATTRIBUTE_NAMES = [
    "Flags",
    "Density",
    "Initial Size",
    "Viscosity Dynamic",
    "Viscosity Bulk",
    "Youngs Modulus",
    "Poissons Ratio",
    "Sand Alpha",
    "Bulk Modulus",
    "Exponent",
    "Elastic Energy",
    "Collider Bits",
    "Position Gradient",
    "Velocity",
    "Velocity Gradient",
    "Initial Position",
    "Goal Position",
]


def node_modifiers(obj: bpy.types.Object) -> dict[str, str]:
    return {
        modifier.name: modifier.node_group.name
        for modifier in obj.modifiers
        if modifier.type == "NODES"
    }


def driver_targets(obj: bpy.types.Object) -> set[bpy.types.ID]:
    if obj.animation_data is None:
        return set()
    return {
        target.id
        for fcurve in obj.animation_data.drivers
        for variable in fcurve.driver.variables
        for target in variable.targets
    }


def new_input(obj_name: str, **overrides) -> dict:
    return {
        "name": obj_name,
        "obj_name": obj_name,
        "problem": "",
        "input_type": "Particles",
        "add_default_generation": True,
        "add_output_object": True,
        "add_default_visualization": True,
        "alembic_attributes": [],
        "selected_alembic_attribute": 0,
        **overrides,
    }


class InputTestCase(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.sim_obj = api().add_simulation("Jelly")
        self.props_collection = bpy.data.collections.new("Props")
        bpy.context.scene.collection.children.link(self.props_collection)
        self.ball = bpy.data.objects.new("Ball", bpy.data.meshes.new("Ball"))
        self.props_collection.objects.link(self.ball)


class AddParticlesTest(InputTestCase):
    def setUp(self):
        super().setUp()
        self.output_obj = api().add_input(self.sim_obj, self.ball, api().PARTICLES)

    def test_makes_the_object_an_input(self):
        self.assertEqual(api().input_objects(self.sim_obj), [self.ball])
        self.assertTrue(api().is_input(self.ball))
        self.assertEqual(self.ball.squishy_volumes.input_type, "Particles")

    def test_generates_particles_driven_by_the_simulation(self):
        self.assertEqual(
            node_modifiers(self.ball),
            {"Squishy Volumes Input": "Squishy Volumes Generate Particles"},
        )
        self.assertEqual(driver_targets(self.ball), {self.sim_obj})

    def test_offers_all_alembic_attributes_disabled(self):
        attributes = self.ball.squishy_volumes.alembic_attributes
        self.assertEqual(
            [attribute.name for attribute in attributes], PARTICLE_ATTRIBUTE_NAMES
        )
        self.assertFalse(any(attribute.enabled for attribute in attributes))

    def test_output_object_loads_the_simulation_cache(self):
        self.assertEqual(self.output_obj.name, "Ball - Output")
        self.assertEqual(self.output_obj.type, "POINTCLOUD")
        [cache_modifier] = [
            modifier
            for modifier in self.output_obj.modifiers
            if modifier.type == "MESH_SEQUENCE_CACHE"
        ]
        self.assertEqual(
            cache_modifier.cache_file, self.sim_obj.squishy_volumes.cache_file
        )
        self.assertEqual(cache_modifier.object_path, "/Ball")

    def test_output_object_is_visualized_next_to_the_input(self):
        self.assertEqual(
            node_modifiers(self.output_obj),
            {"Squishy Volumes Output": "Squishy Volumes Visualize Particles"},
        )
        self.assertEqual(
            list(self.output_obj.users_collection), [self.props_collection]
        )

    def test_output_object_is_not_an_input(self):
        self.assertFalse(api().is_input(self.output_obj))


class AddColliderTest(InputTestCase):
    def test_generates_a_collider_without_output(self):
        objects_before = set(bpy.data.objects)

        output_obj = api().add_input(self.sim_obj, self.ball, api().COLLIDER)

        self.assertIsNone(output_obj)
        self.assertEqual(set(bpy.data.objects), objects_before)
        self.assertEqual(self.ball.squishy_volumes.input_type, "Collider")
        self.assertEqual(
            node_modifiers(self.ball),
            {"Squishy Volumes Input": "Squishy Volumes Generate Collider"},
        )
        self.assertEqual(len(self.ball.squishy_volumes.alembic_attributes), 0)


class AddInputOptionsTest(InputTestCase):
    def test_without_default_generation(self):
        api().add_input(self.sim_obj, self.ball, default_generation=False)

        self.assertEqual(len(self.ball.modifiers), 0)
        self.assertEqual(api().input_objects(self.sim_obj), [self.ball])

    def test_without_output_object(self):
        objects_before = set(bpy.data.objects)

        output_obj = api().add_input(self.sim_obj, self.ball, output_object=False)

        self.assertIsNone(output_obj)
        self.assertEqual(set(bpy.data.objects), objects_before)

    def test_without_default_visualization(self):
        output_obj = api().add_input(
            self.sim_obj, self.ball, default_visualization=False
        )

        self.assertEqual(node_modifiers(output_obj), {})


class AddInputAgainTest(InputTestCase):
    def test_adds_another_output_without_another_generator(self):
        first_output = api().add_input(self.sim_obj, self.ball)

        second_output = api().add_input(
            self.sim_obj, self.ball, default_generation=False
        )

        self.assertNotEqual(second_output, first_output)
        self.assertEqual(
            {output.name for output in (first_output, second_output)},
            {"Ball - Output", "Ball - Output.001"},
        )
        self.assertEqual(
            node_modifiers(self.ball),
            {"Squishy Volumes Input": "Squishy Volumes Generate Particles"},
        )
        self.assertEqual(api().input_objects(self.sim_obj), [self.ball])


class InvalidInputTest(InputTestCase):
    def assert_rejected(self, obj, *args):
        with self.assertRaises(api().InvalidInput):
            api().add_input(self.sim_obj, obj, *args)

    def test_rejects_objects_that_are_not_meshes(self):
        empty = bpy.data.objects.new("Empty", None)
        bpy.context.scene.collection.objects.link(empty)

        self.assert_rejected(empty)
        self.assertFalse(api().is_input(empty))

    def test_rejects_unknown_input_types(self):
        self.assert_rejected(self.ball, "Jelly Beans")
        self.assertFalse(api().is_input(self.ball))
        self.assertEqual(len(self.ball.modifiers), 0)

    def test_rejects_simulation_objects(self):
        self.assert_rejected(self.sim_obj)

    def test_errors_are_squishy_volumes_errors(self):
        self.assertTrue(issubclass(api().InvalidInput, api().SquishyVolumesError))


class RemoveInputTest(InputTestCase):
    def test_keeps_the_object_and_its_modifiers(self):
        api().add_input(self.sim_obj, self.ball)

        api().remove_input(self.ball)

        self.assertEqual(api().input_objects(self.sim_obj), [])
        self.assertIn("Squishy Volumes Input", self.ball.modifiers)
        self.assertIsNone(api().input_problem(self.ball))


class CopyAlembicAttributesTest(InputTestCase):
    def setUp(self):
        super().setUp()
        api().add_input(self.sim_obj, self.ball, output_object=False)
        for attribute in self.ball.squishy_volumes.alembic_attributes:
            attribute.enabled = attribute.name in {"Velocity", "Density"}
        self.other_ball = add_mesh_object("Other Ball")
        api().add_input(self.sim_obj, self.other_ball, output_object=False)
        self.ground = add_mesh_object("Ground")
        api().add_input(self.sim_obj, self.ground, api().COLLIDER)

    def test_copies_to_inputs_of_the_same_type(self):
        updated = api().copy_alembic_attributes(
            self.ball, [self.ball, self.other_ball, self.ground]
        )

        self.assertEqual(updated, [self.other_ball])
        enabled = {
            attribute.name
            for attribute in self.other_ball.squishy_volumes.alembic_attributes
            if attribute.enabled
        }
        self.assertEqual(enabled, {"Velocity", "Density"})
        self.assertEqual(len(self.ground.squishy_volumes.alembic_attributes), 0)


class InputOperatorTest(InputTestCase):
    def test_adds_the_listed_objects_and_skips_problems(self):
        empty = bpy.data.objects.new("Empty", None)
        bpy.context.scene.collection.objects.link(empty)
        self.ball.select_set(True)

        bpy.ops.scene.squishy_volumes_add_input_objects(
            uuid=self.sim_obj.squishy_volumes.uuid,
            inputs=[new_input("Ball", input_type="Collider"), new_input("Empty")],
        )

        self.assertEqual(api().input_objects(self.sim_obj), [self.ball])
        self.assertEqual(self.ball.squishy_volumes.input_type, "Collider")

    def test_remove_unknown_object_raises_in_scripts(self):
        with self.assertRaisesRegex(RuntimeError, "no object named"):
            bpy.ops.object.squishy_volumes_remove_input_object(name="Nothing")
