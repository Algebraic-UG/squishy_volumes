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

from typing import TYPE_CHECKING, Any, Literal

from nodebpy import TreeBuilder
from nodebpy import geometry as g
from nodebpy.builder import (
    BooleanSocket,
    CustomGeometryGroup,
    FloatSocket,
    GeometrySocket,
    SocketAccessor,
    StringSocket,
    VectorSocket,
)
from nodebpy.types import (
    InputAny,
    InputFloat,
    InputGeometry,
    InputString,
    InputVector,
)

_DataType = Literal[
    "FLOAT",
    "INT",
    "BOOLEAN",
    "FLOAT_VECTOR",
    "FLOAT_COLOR",
    "QUATERNION",
    "FLOAT4X4",
]

_INTERFACE_METHOD = {
    "FLOAT": "float",
    "INT": "integer",
    "BOOLEAN": "boolean",
    "FLOAT_VECTOR": "vector",
    "FLOAT_COLOR": "color",
    "QUATERNION": "rotation",
    "FLOAT4X4": "matrix",
}


class NamedAttributeWithDefaultAndWarn[T](CustomGeometryGroup):
    _name = "Named Attribute With Default And Warn"
    _data_type: _DataType = "FLOAT"

    class _Inputs[S](SocketAccessor):
        geometry: GeometrySocket
        name: StringSocket
        default: S

    class _Outputs[S](SocketAccessor):
        attribute: S

    if TYPE_CHECKING:

        @property
        def i(self) -> _Inputs[T]: ...
        @property
        def o(self) -> _Outputs[T]: ...

    def __init__(
        self,
        geometry: InputGeometry = None,
        name: InputString = "",
        default: InputAny = None,
        *,
        data_type: _DataType = "FLOAT",
    ):
        self._data_type = data_type
        super().__init__(Geometry=geometry, Name=name, Default=default)

    def _group_name(self) -> str:
        return f"{self._name} ({self._data_type})"

    def _build_group(self, tree: TreeBuilder[Any]) -> None:
        geometry = tree.inputs.geometry("Geometry")
        name = tree.inputs.string("Name")
        method = _INTERFACE_METHOD[self._data_type]
        default = getattr(tree.inputs, method)("Default")

        message = g.JoinStrings(["Consider adding the output attr '", name, "'"])
        attribute = g.NamedAttribute(name, data_type=self._data_type)
        missing = (
            g.SampleIndex.point.boolean(geometry=geometry, value=attribute.o.exists)
            >> g.BooleanMath.l_not()
            >> g.Warning(message=message, warning_type="WARNING")
        ).o.show
        assert isinstance(missing, BooleanSocket)
        getattr(missing.switch, method)(
            false=attribute.o.attribute, true=default
        ) >> getattr(tree.outputs, method)("Attribute")

    @classmethod
    def float(
        cls,
        geometry: InputGeometry = None,
        name: InputString = "",
        default: InputFloat = 0.0,
    ) -> "NamedAttributeWithDefaultAndWarn[FloatSocket]":
        return NamedAttributeWithDefaultAndWarn(
            geometry, name, default, data_type="FLOAT"
        )

    @classmethod
    def vector(
        cls,
        geometry: InputGeometry = None,
        name: InputString = "",
        default: InputVector = (0.0, 0.0, 0.0),
    ) -> "NamedAttributeWithDefaultAndWarn[VectorSocket]":
        return NamedAttributeWithDefaultAndWarn(
            geometry, name, default, data_type="FLOAT_VECTOR"
        )


def float_attribute(
    geometry: InputGeometry, name: str, default: InputFloat
) -> FloatSocket:
    return NamedAttributeWithDefaultAndWarn.float(geometry, name, default).o.attribute


def vector_attribute(
    geometry: InputGeometry, name: str, default: InputVector
) -> VectorSocket:
    return NamedAttributeWithDefaultAndWarn.vector(geometry, name, default).o.attribute
