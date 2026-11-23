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
import mathutils

from .support import AddonTestCase, add_mesh_object, addon


class RestrictViewTest(AddonTestCase):
    def setUp(self):
        super().setUp()
        self.obj = add_mesh_object("Points")
        self.obj.data.from_pydata([(0, 0, 0), (1.5, 0, 0), (0, 0, -2.5)], [], [])
        self.view = bpy.data.objects.new("View", None)
        bpy.context.scene.collection.objects.link(self.view)
        modifier = self.obj.modifiers.new("Restrict View", type="NODES")
        modifier.node_group = addon().assets.create_geometry_nodes_restrict_view()
        modifier.properties.inputs.Socket_2.value = self.view

    def remaining_vertices(self) -> int:
        evaluated = self.obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
        return len(evaluated.data.vertices)

    def test_interface(self):
        group = self.obj.modifiers["Restrict View"].node_group
        self.assertEqual(
            [
                (item.in_out, item.name, item.identifier, item.socket_type)
                for item in group.interface.items_tree
            ],
            [
                ("OUTPUT", "Geometry", "Socket_0", "NodeSocketGeometry"),
                ("INPUT", "Geometry", "Socket_1", "NodeSocketGeometry"),
                ("INPUT", "Object", "Socket_2", "NodeSocketObject"),
                ("INPUT", "Invert", "Socket_3", "NodeSocketBool"),
            ],
        )

    def test_keeps_vertices_inside_the_view_cube(self):
        self.assertEqual(self.remaining_vertices(), 1)

    def test_follows_the_view_transform(self):
        self.view.matrix_world = mathutils.Matrix.Diagonal((2, 1, 3, 1))
        self.assertEqual(self.remaining_vertices(), 3)

    def test_invert_keeps_the_vertices_outside_the_view_cube(self):
        self.obj.modifiers["Restrict View"].properties.inputs.Socket_3.value = True
        self.assertEqual(self.remaining_vertices(), 2)
