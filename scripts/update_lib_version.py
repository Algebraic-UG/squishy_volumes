import toml
from pathlib import Path
import sys

scripts_dir = Path(sys.argv[0]).parent
repo_root = scripts_dir / ".."
wrap_dir = repo_root / "rust" / "crates" / "wrap"
cargo_toml_path = wrap_dir / "Cargo.toml"
python_shim_path = repo_root / "python" / "shim.py"
rust_shim_path = wrap_dir / "src" / "shim.rs"

with cargo_toml_path.open("r") as f:
    cargo_toml = toml.load(f)
version = cargo_toml["package"]["version"]
major, minor, patch = version.split(".")

name = "squishy_volumes_wrap"
versioned_name = f"{name}_{major}_{minor}_{patch}"

with python_shim_path.open("w") as f:
    f.write(
        f"""# This file is generated to alleviate https://github.com/Algebraic-UG/squishy_volumes/issues/83

import {versioned_name} as {name}
"""
    )

with rust_shim_path.open("w") as f:
    f.write(
        f"""// This file is generated to alleviate https://github.com/Algebraic-UG/squishy_volumes/issues/83

use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
pub fn {versioned_name}(m: &Bound<'_, PyModule>) -> PyResult<()> {{
    super::{name}(m)
}}
"""
    )

print(f"Updated {python_shim_path} and {rust_shim_path} to {version}.")
