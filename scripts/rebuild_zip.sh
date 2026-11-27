#!/bin/bash

set -e

cd "$(dirname "$0")/.."

scripts_dir=$(realpath scripts)
blender_ext_file="${scripts_dir}/blender_ext.py"
repo_root=$(dirname "${scripts_dir}")
wrap_dir="${repo_root}/rust/crates/wrap"
extension_dir="${repo_root}/python/src/squishy_volumes_extension"
wheels_dir="${extension_dir}/wheels"

rm -f python/src/squishy_volumes_extension/wheels/*.whl

uv run --with toml "${scripts_dir}/update_lib_version.py"

"${scripts_dir}/add_license_headers.sh"

(
    cd "${repo_root}/python"
    uv run nodebpy build
)

cd "${wrap_dir}"
# a separate target dir keeps the library watched by hot reloading in target/release untouched
uvx --python 3.12 maturin build --no-default-features --release --target-dir "${repo_root}/rust/target/zip" --out "${wheels_dir}"

cd -
uv run --with toml "${scripts_dir}/update_manifest.py"

test -f "${blender_ext_file}" || wget https://raw.githubusercontent.com/blender/blender/refs/tags/v5.0.1/scripts/addons_core/bl_pkg/cli/blender_ext.py --output-document="${blender_ext_file}"

cd "${extension_dir}"
uv run --python 3.13 "${blender_ext_file}" build
uv run --python 3.13 "${blender_ext_file}" server-generate --repo-dir=./
