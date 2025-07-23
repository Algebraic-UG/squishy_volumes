import toml
from pathlib import Path
import sys

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
cargo_toml_path = repo_root / "rust" / "crates" / "wrap" / "Cargo.toml"
manifest_template_path = scripts_dir / "blender_manifest_template.toml"
extension_dir = repo_root / "python"
wheels_dir = extension_dir / "wheels"
manifest_path = extension_dir / "blender_manifest.toml"

with cargo_toml_path.open("r") as f:
    cargo_toml = toml.load(f)
version = cargo_toml["package"]["version"]

wheel_paths = sorted(wheels_dir.glob("*.whl"))
if not wheel_paths:
    print(f"Found no wheels in {wheels_dir}", file=sys.stderr)
    sys.exit(1)

with manifest_template_path.open("r") as f:
    manifest = toml.load(f)

manifest["version"] = version
manifest["wheels"] = [f"./wheels/{wheel.name}" for wheel in wheel_paths]

with manifest_path.open("w") as f:
    toml.dump(manifest, f)

print(f"Updated {manifest_path} to {version} with {len(wheel_paths)} wheel(s).")
