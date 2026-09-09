# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

from pathlib import Path

import bpy  # ty: ignore[unresolved-import]


def _load_material(file_stem: str, name: str) -> bpy.types.Material:
    file_path = Path(__file__).parent / f"{file_stem}.blend"
    with bpy.data.libraries.load(str(file_path), link=True, pack=True) as (
        _data_src,
        data_dst,
    ):
        data_dst.materials = [name]
    return bpy.data.materials[name]


def _load_tree(file_stem: str, name: str) -> bpy.types.NodeTree:
    file_path = Path(__file__).parent / f"{file_stem}.blend"
    with bpy.data.libraries.load(str(file_path), link=True, pack=True) as (
        _data_src,
        data_dst,
    ):
        data_dst.node_groups = [name]

    return bpy.data.node_groups[name]


def create_material_display_uvw() -> bpy.types.Material:
    return _load_material("material_display_uvw", "Squishy Volumes Display UVW")


def create_geometry_nodes_grid() -> bpy.types.NodeTree:
    return _load_tree("geometry_nodes_grid_momentum", "Squishy Volumes Grid Momentum")


def create_geometry_nodes_particles() -> bpy.types.NodeTree:
    return _load_tree("geometry_nodes_particles", "Squishy Volumes Particle")


def create_geometry_nodes_restrict_view() -> bpy.types.NodeTree:
    return _load_tree("geometry_nodes_restrict_view", "Squishy Volumes Restrict View")


def create_geometry_nodes_generate_particles() -> bpy.types.NodeTree:
    return _load_tree(
        "geometry_nodes_generate_particles", "Squishy Volumes Generate Particles"
    )


def create_geometry_nodes_generate_collider() -> bpy.types.NodeTree:
    return _load_tree("geometry_nodes_generate_collider", "Generate Collider")


def create_geometry_nodes_generate_goal_positions() -> bpy.types.NodeTree:
    return _load_tree(
        "geometry_nodes_generate_goal_positions", "Squishy Volumes Set Goals"
    )
