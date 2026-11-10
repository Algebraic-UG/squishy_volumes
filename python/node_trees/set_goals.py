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
from ._shared.inside import is_inside_object
from ._shared.matrix import self_matrix


class SetGoals(CustomGeometryGroup):
    _name = "Squishy Volumes Set Goals"
    _tree_properties: ClassVar[dict[str, Any]] = {"is_modifier": True}

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry()
        choose = tree.inputs.object("Choose")
        move = tree.inputs.object("Move")

        with g.Frame(label="Detect Choosen Particles"):
            inside = is_inside_object(choose.geometry("RELATIVE"), g.Position())

        with g.Frame(label="Local -> Chooser -> Mover -> Back to Local"):
            goal_position = g.Position().o.position.transform(
                self_matrix().invert()
                @ move.matrix()
                @ choose.matrix().invert()
                @ self_matrix()
            )
        with g.Frame(label="Store Flag"):
            inside = inside >> g.Reroute()
            geometry = (
                geometry
                >> g.StoreNamedAttribute.point.boolean(
                    selection=inside,
                    name=names.inputs.PARTICLE_HAS_GOAL.name,
                    value=True,
                )
                >> g.SetPosition(selection=inside, position=goal_position)
            )

        with g.Frame(label="Store Goal"):
            (
                geometry
                >> g.StoreNamedAttribute.point.vector(
                    name=names.inputs.PARTICLE_GOAL_POSITION.name,
                    value=g.Position().o.position.transform(self_matrix()),
                )
                >> tree.outputs.geometry()
            )


ASSET = SetGoals
