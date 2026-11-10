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

import sys
from pathlib import Path

import bpy
from tree_clipper.import_nodes import ImportIntermediate, ImportParameters, ImportReport
from tree_clipper.import_to_asset_file import import_to_asset_file
from tree_clipper.specific_handlers import BUILT_IN_IMPORTER

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
asset_dir = repo_root / "python" / "src" / "squishy_volumes_extension" / "assets"
tree_clipper_jsons = repo_root / "tree_clipper_jsons"


def _load_tree_clipper(
    file_path: Path,
    externals: list[tuple[int, bpy.types.ID]],
) -> ImportReport:
    print(f"Importing {file_path}")
    intermediate = ImportIntermediate(file_path=file_path)
    intermediate.set_external(iter(externals))
    report = intermediate.import_all(
        ImportParameters(
            specific_handlers=BUILT_IN_IMPORTER,
            debug_prints=False,
        )
    )
    for warning in report.warnings:
        print(f"warning: {warning}")
    return report


def _load_tree_clipper_to_asset(
    file_path: Path,
    externals: list[tuple[int, bpy.types.ID]],
):
    print(f"Importing to asset {file_path}")
    intermediate = ImportIntermediate(file_path=file_path)
    intermediate.set_external(iter(externals))
    report, _asset_file_path = import_to_asset_file(
        import_intermediate=intermediate,
        parameters=ImportParameters(
            specific_handlers=BUILT_IN_IMPORTER,
            debug_prints=False,
        ),
        asset_file_path=asset_dir / f"{file_path.stem}.blend",
    )
    for warning in report.warnings:
        print(f"warning: {warning}")


colored_instances_name = _load_tree_clipper(
    tree_clipper_jsons / "material_colored_instances.json", []
).rename_material[1]

_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_grid_momentum.json",
    [(331, bpy.data.materials[colored_instances_name])],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_restrict_view.json",
    [],
)
