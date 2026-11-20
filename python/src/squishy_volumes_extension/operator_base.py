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

from .api import find_simulation
from .errors import ScaledInputError
from .popup import popup


def report_error(operator, uuid: str, error: RuntimeError):
    popup(uuid, str(error), isinstance(error, ScaledInputError))
    operator.report({"ERROR"}, str(error))


class ReportingOperator:
    popup_uuid = ""

    def execute(self, context):
        try:
            return self.run(context)
        except RuntimeError as error:
            report_error(self, self.popup_uuid, error)
            return {"CANCELLED"}

    def run(self, context) -> set[str]:
        raise NotImplementedError


class SimulationOperator(ReportingOperator):
    uuid: bpy.props.StringProperty()

    @property
    def popup_uuid(self) -> str:
        return self.uuid

    @property
    def sim_obj(self) -> bpy.types.Object:
        return find_simulation(self.uuid)
