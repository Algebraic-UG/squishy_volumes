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

from .._shared.attributes import float_attribute


class ReconstructedSurfaceVisualization(CustomGeometryGroup):
    _name = "Reconstructed Surface Visualization"

    def __init__(
        self,
        geometry: InputGeometry = ...,
        smooth_shading: InputBoolean = True,
        smooth_iterations: InputInteger = 1,
        override_radii: InputBoolean = False,
        override_radii_value: InputFloat = 0.25,
    ):
        super().__init__(
            **{
                "Geometry": geometry,
                "Smooth Shading": smooth_shading,
                "Smooth Iterations": smooth_iterations,
                "Override Radii": override_radii,
                "Override Radii Value": override_radii_value,
            }
        )

    def _build_group(self, tree: TreeBuilder[GeometryNodeTree]):
        geometry = tree.inputs.geometry("Geometry")
        with tree.inputs.panel("Smooth"):
            smooth_shading = tree.inputs.boolean("Smooth Shading")
            smooth_iterations = tree.inputs.integer("Smooth Iterations")

        with tree.inputs.panel("Override Radii"):
            override_radii = tree.inputs.boolean("Override Radii", is_panel_toggle=True)
            override_radii_value = tree.inputs.float("Override Radii Value")

        with g.Frame("Radii of the points"):
            radius = override_radii.switch.float(
                false=float_attribute(geometry, "initial_size", 1.0),
                true=override_radii_value * 2.0,
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
