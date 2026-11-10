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
from nodebpy.builder import BooleanSocket, CustomGeometryGroup, IntegerSocket
from nodebpy.types import InputGeometry, InputInteger, InputMenu, InputVector

from . import names
from .attributes import vector_attribute


# Workaround until int attributes can be read
# see https://projects.blender.org/blender/blender/pulls/165562
def vector_2d_to_int(vector: InputVector) -> IntegerSocket:
    low, high = [c.to_integer("FLOOR") for c in vector[:2]]
    return g.BitMath.l_or(low, g.BitMath.shift(high, 16)).o.value


def is_bit_set(bits: InputInteger, bit: InputInteger) -> BooleanSocket:
    return g.Compare.integer.not_equal(
        g.BitMath.l_and(bits, g.BitMath.shift(1, bit)), 0
    ).o.result


class GetFlag(CustomGeometryGroup):
    _name = "Get Flag"

    def __init__(self, geometry: InputGeometry = ..., flag: InputMenu = "FAILED"):
        super().__init__(Geometry=geometry, Flag=flag)

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        flags = vector_attribute(
            tree.inputs.geometry("Geometry"), names.outputs.FLAGS.name, (0, 0, 0)
        )
        bit = tree.inputs.menu("Flag") >> g.MenuSwitch.integer(
            items=names.outputs.PARTICLE_FLAG_BITS
        )
        is_bit_set(vector_2d_to_int(flags), bit) >> tree.outputs.boolean()
