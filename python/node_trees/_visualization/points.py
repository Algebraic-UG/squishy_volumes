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

from .._shared import names
from .._shared.attributes import float_attribute
from .._shared.flags import GetFlag
from .interface import FAILED_SCALE, SCALE


class PointVisualization(CustomGeometryGroup):
    _name = "Point Visualization"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        scale: InputFloat = SCALE.default,
        failed_scale: InputFloat = FAILED_SCALE.default,
    ):
        super().__init__(
            **{"Geometry": geometry, SCALE.name: scale, FAILED_SCALE.name: failed_scale}
        )

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry("Geometry")
        scale = GetFlag(geometry, "FAILED").o.boolean.switch.float(
            SCALE.add(tree), FAILED_SCALE.add(tree)
        )
        size = float_attribute(geometry, names.outputs.INITIAL_SIZE.name, 1)
        (
            geometry
            >> g.SetPointRadius(radius=0.5 * scale * size)
            >> tree.outputs.geometry()
        )
