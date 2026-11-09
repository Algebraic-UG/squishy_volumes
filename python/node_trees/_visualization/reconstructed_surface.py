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
from nodebpy.types import InputBoolean, InputFloat, InputGeometry, InputInteger

from .._shared import names
from .._shared.attributes import float_attribute
from .interface import (
    RADIUS_SCALE,
    SMOOTH_ITERATIONS,
    SMOOTH_SHADING,
    add_reconstruction_inputs,
)


class ReconstructedSurfaceVisualization(CustomGeometryGroup):
    _name = "Reconstructed Surface Visualization"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        radius_scale: InputFloat = RADIUS_SCALE.default,
        smooth_shading: InputBoolean = SMOOTH_SHADING.default,
        smooth_iterations: InputInteger = SMOOTH_ITERATIONS.default,
    ):
        super().__init__(
            **{
                "Geometry": geometry,
                RADIUS_SCALE.name: radius_scale,
                SMOOTH_SHADING.name: smooth_shading,
                SMOOTH_ITERATIONS.name: smooth_iterations,
            }
        )

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry("Geometry")
        radius_scale, smooth_shading, smooth_iterations = add_reconstruction_inputs(
            tree
        )

        with g.Frame("Radii of the points"):
            radius = (
                radius_scale
                * 0.5
                * float_attribute(geometry, names.outputs.INITIAL_SIZE.name, 1.0)
            )

        with g.Frame("Estimate a good voxel size"):
            voxel_size = (
                g.AttributeStatistic(geometry=geometry, attribute=radius).o.mean * 0.5
            )

        with g.Frame("Points -> SDF -> Smooth -> Mesh"):
            geometry = (
                g.PointsToSDFGrid(geometry, radius, voxel_size)
                >> g.SDFGridMean(iterations=smooth_iterations)
                >> g.GridToMesh(threshold=0)
            )

        (
            geometry
            >> g.SetShadeSmooth(shade_smooth=smooth_shading)
            >> tree.outputs.geometry()
        )
