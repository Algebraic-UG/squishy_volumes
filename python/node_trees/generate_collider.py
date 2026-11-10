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
from nodebpy.builder import CustomGeometryGroup

from ._shared import names
from ._shared.matrix import self_matrix


class GenerateCollider(CustomGeometryGroup):
    _name = "Squishy Volumes Generate Collider"
    _tree_properties: ClassVar[dict[str, Any]] = {"is_modifier": True}

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry()
        friction = tree.inputs.float(
            "Friction",
            0.3,
            description="Coulomb friction factor.",
            min_value=0.0,
            max_value=1.0,
            subtype="FACTOR",
        )
        damping = tree.inputs.float(
            "Damping",
            0.1,
            description="Damps bounce in normal direction",
            min_value=0.0,
            max_value=1.0,
            subtype="FACTOR",
        )

        (
            geometry
            >> g.Triangulate()
            >> g.StoreNamedAttribute.point.vector(
                name=names.inputs.COLLIDER_POSITION.name,
                value=g.Position().o.position.transform(self_matrix()),
            )
            >> g.StoreNamedAttribute.face.float(
                name=names.inputs.COLLIDER_FRICTION.name, value=friction
            )
            >> g.StoreNamedAttribute.face.float(
                name=names.inputs.COLLIDER_DAMPING.name, value=damping
            )
            >> tree.outputs.geometry()
        )


ASSET = GenerateCollider
