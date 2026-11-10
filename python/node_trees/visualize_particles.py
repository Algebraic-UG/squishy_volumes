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

from ._visualization.deformed_cubes import DeformedCubesVisualization
from ._visualization.interface import FAILED_SCALE, SCALE, add_reconstruction_inputs
from ._visualization.points import PointVisualization
from ._visualization.reconstructed_surface import ReconstructedSurfaceVisualization


class VisualizeParticles(CustomGeometryGroup):
    _name = "Squishy Volumes Visualize Particles"
    _tree_properties: ClassVar[dict[str, Any]] = {"is_modifier": True}

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry()
        display_as = tree.inputs.menu("Display As", "Points")
        scale = SCALE.add(tree)
        failed_scale = FAILED_SCALE.add(tree)
        reconstruction_inputs = add_reconstruction_inputs(tree)

        (
            g.MenuSwitch.geometry(
                display_as,
                {
                    "Points": PointVisualization(geometry, scale, failed_scale),
                    "Deformed Cubes": DeformedCubesVisualization(geometry, scale),
                    "Reconstructed": ReconstructedSurfaceVisualization(
                        geometry, *reconstruction_inputs
                    ),
                },
            )
            >> tree.outputs.geometry()
        )


ASSET = VisualizeParticles
