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

import bpy


from pathlib import Path

from .._vendor.tree_clipper.specific_handlers import BUILT_IN_IMPORTER
from .._vendor.tree_clipper.import_nodes import (
    ImportReport,
    ImportIntermediate,
    ImportParameters,
)


def _load_tree_clipper(
    name: str,
    externals: list[tuple[int, bpy.types.ID]],
) -> ImportReport:
    file_path = Path(__file__).parent / name
    intermediate = ImportIntermediate(file_path=file_path)
    intermediate.set_external(iter(externals))
    report = intermediate.import_all(
        ImportParameters(
            specific_handlers=BUILT_IN_IMPORTER,
            debug_prints=False,
        )
    )
    for warning in report.warnings:
        print(f"Tree Clipper: {name}, warning: {warning}")
    return report


def _load_tree_clipper_tree(
    name: str,
    externals: list[tuple[int, bpy.types.ID]] = [],
) -> bpy.types.NodeTree:
    return _load_tree_clipper(name, externals).last_getter()  # ty:ignore[call-non-callable, invalid-return-type]


def _load_tree_clipper_material(
    name: str,
    externals: list[tuple[int, bpy.types.ID]] = [],
) -> bpy.types.Material:
    report = _load_tree_clipper(name, externals)
    return bpy.data.materials[report.rename_material[1]]  # ty:ignore[not-subscriptable]


def create_material_colored_instances() -> bpy.types.Material:
    return _load_tree_clipper_material("material_colored_instances.json")


def create_material_display_uvw() -> bpy.types.Material:
    return _load_tree_clipper_material("material_display_uvw.json")


def create_geometry_nodes_grid_distance() -> bpy.types.NodeTree:
    colored_instances = create_material_colored_instances()
    return _load_tree_clipper_tree(
        "geometry_nodes_grid_distance.json",
        [(389, colored_instances)],
    )


def create_geometry_nodes_grid_momentum() -> bpy.types.NodeTree:
    colored_instances = create_material_colored_instances()
    return _load_tree_clipper_tree(
        "geometry_nodes_grid_momentum.json",
        [(331, colored_instances)],
    )


def create_geometry_nodes_particles() -> bpy.types.NodeTree:
    colored_instances = create_material_colored_instances()
    return _load_tree_clipper_tree(
        "geometry_nodes_particles.json",
        [(702, colored_instances)],
    )


def create_geometry_nodes_surface_samples() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_surface_samples.json")


def create_geometry_nodes_store_reference() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_store_reference.json")


def create_geometry_nodes_move_with_reference() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_move_with_reference.json")


def create_geometry_nodes_store_breaking_frame():
    raise RuntimeError("Not implemented yet")


def create_geometry_nodes_remove_broken():
    raise RuntimeError("Not implemented yet")


def create_geometry_nodes_restrict_view() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_restrict_view.json")


def create_geometry_nodes_generate_particles() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_generate_particles.json")


def create_geometry_nodes_generate_collider() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_generate_collider.json")


def create_geometry_nodes_generate_goal_positions() -> bpy.types.NodeTree:
    return _load_tree_clipper_tree("geometry_nodes_generate_goal_positions.json")
