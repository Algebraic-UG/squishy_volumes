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

from bpy.types import GeometryNodeTree
from nodebpy import TreeBuilder
from nodebpy import geometry as g
from nodebpy.builder import BooleanSocket, CustomGeometryGroup
from nodebpy.types import InputGeometry, InputInteger, InputVector


class IsInsideObject(CustomGeometryGroup):
    _name = "Is Inside Object"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        position: InputVector = (0.0, 0.0, 0.0),
        seed: InputInteger = 0,
    ):
        super().__init__(Geometry=geometry, Position=position, Seed=seed)

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry("Geometry")
        bb = geometry >> g.BoundingBox()
        extents = bb.o.max - bb.o.min

        direction = g.RandomValue.vector(
            min=-1,
            max=1.0,
            seed=tree.inputs.integer("Seed"),
        )
        raycast = g.Raycast(
            target_geometry=geometry,
            source_position=tree.inputs.vector("Position"),
            ray_direction=direction,
            ray_length=extents.x + extents.y + extents.z,
        )

        (
            raycast.o.is_hit & (raycast.o.hit_normal.dot(direction) > 0)
        ) >> tree.outputs.boolean("Inside")


def is_inside_object(geometry: InputGeometry, position: InputVector) -> BooleanSocket:
    return IsInsideObject(geometry, position, 0) & IsInsideObject(geometry, position, 1)
