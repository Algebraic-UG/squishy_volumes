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

from .errors import InvalidInput, SimulationNotFound, SquishyVolumesError
from .inputs import (
    COLLIDER,
    PARTICLES,
    add_input,
    copy_alembic_attributes,
    input_problem,
    is_input,
    remove_input,
    reset_alembic_attributes,
)
from .simulation import (
    add_simulation,
    clone_simulation,
    find_simulation,
    input_objects,
    reload,
    reload_all,
    remove_simulation,
    simulations,
)

__all__ = [
    "COLLIDER",
    "PARTICLES",
    "InvalidInput",
    "SimulationNotFound",
    "SquishyVolumesError",
    "add_input",
    "add_simulation",
    "clone_simulation",
    "copy_alembic_attributes",
    "find_simulation",
    "input_objects",
    "input_problem",
    "is_input",
    "reload",
    "reload_all",
    "remove_input",
    "remove_simulation",
    "reset_alembic_attributes",
    "simulations",
]
