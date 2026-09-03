import bpy
import sys
from pathlib import Path
import tree_clipper
from tree_clipper.import_to_asset_file import import_to_asset_file
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
    report, asset_file_path = import_to_asset_file(
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
    tree_clipper_jsons / "material_display_uvw.json",
    [],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_grid_momentum.json",
    [(331, bpy.data.materials[colored_instances_name])],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_particles.json",
    [(1232, bpy.data.materials[colored_instances_name])],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_restrict_view.json",
    [],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_generate_particles.json",
    [],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_generate_collider.json",
    [],
)
_load_tree_clipper_to_asset(
    tree_clipper_jsons / "geometry_nodes_generate_goal_positions.json",
    [],
)
