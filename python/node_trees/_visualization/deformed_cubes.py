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
from nodebpy.builder import CustomGeometryGroup
from nodebpy.types import InputFloat, InputGeometry

from .._shared.attributes import float_attribute, vector_attribute
from .._shared.matrix import matrix_from_columns


class DeformedCubesVisualization(CustomGeometryGroup):
    _name = "Deformed Cubes Visualization"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        scale: InputFloat = 1.0,
    ):
        super().__init__(Geometry=geometry, Scale=scale)

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry("Geometry")
        scale = tree.inputs.float("Scale")

        size = scale * float_attribute(geometry, "initial_size", 1)
        deformation = matrix_from_columns(
            vector_attribute(geometry, "position_gradient_x", (1, 0, 0)),
            vector_attribute(geometry, "position_gradient_y", (0, 1, 0)),
            vector_attribute(geometry, "position_gradient_z", (0, 0, 1)),
            g.Position().o.position,
        )

        (
            g.InstanceOnPoints(points=geometry, instance=g.Cube())
            >> g.SetInstanceTransform(
                transform=deformation @ g.CombineTransform(scale=size)
            )
            >> tree.outputs.geometry()
        )
