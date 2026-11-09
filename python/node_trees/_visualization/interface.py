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

from typing import Any

from nodebpy import TreeBuilder

from .._shared.inputs import Input

SCALE = Input("float", "Scale", 1.0, {"min_value": 0.0})
FAILED_SCALE = Input(
    "float",
    "Failed Scale",
    5.0,
    {
        "min_value": 0.0,
        "max_value": 10_000.0,
        "description": "How much larger failed particles are scaled.",
    },
)
SMOOTH_SHADING = Input("boolean", "Smooth Shading", True)
SMOOTH_ITERATIONS = Input("integer", "Smooth Iterations", 1, {"min_value": 0})
OVERRIDE_RADII = Input("boolean", "Override Radii", False, {"is_panel_toggle": True})
OVERRIDE_RADII_VALUE = Input(
    "float",
    "Override Radii Value",
    0.25,
    {"min_value": 0.0, "subtype": "DISTANCE"},
)


def add_reconstruction_inputs(tree: TreeBuilder) -> tuple[Any, Any, Any, Any]:
    with tree.inputs.panel("Smooth"):
        smooth_shading = SMOOTH_SHADING.add(tree)
        smooth_iterations = SMOOTH_ITERATIONS.add(tree)
    with tree.inputs.panel("Override Radii"):
        override_radii = OVERRIDE_RADII.add(tree)
        override_radii_value = OVERRIDE_RADII_VALUE.add(tree)
    return smooth_shading, smooth_iterations, override_radii, override_radii_value
