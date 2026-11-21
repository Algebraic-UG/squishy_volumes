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

from typing import Any, ClassVar

from bpy.types import GeometryNodeTree
from nodebpy import TreeBuilder
from nodebpy import geometry as g
from nodebpy.builder import BooleanSocket, CustomGeometryGroup, VectorSocket

from ._shared.matrix import self_matrix


def is_outside_unit_cube(position: VectorSocket) -> BooleanSocket:
    x, y, z = [(c > 1.0) | (c < -1.0) for c in position]
    return x | y | z


class RestrictView(CustomGeometryGroup):
    _name = "Squishy Volumes Restrict View"
    _tree_properties: ClassVar[dict[str, Any]] = {"is_modifier": True}

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        # Declared first so the object input keeps the identifier Socket_2
        output = tree.outputs.geometry()
        geometry = tree.inputs.geometry()
        view = tree.inputs.object("Object")

        position_in_view = g.Position().o.position.transform(
            view.matrix().invert() @ self_matrix()
        )
        (
            geometry
            >> g.DeleteGeometry.point(selection=is_outside_unit_cube(position_in_view))
            >> output
        )


ASSET = RestrictView
