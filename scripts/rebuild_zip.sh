#!/bin/bash

set -e

scripts_dir=$(dirname $(realpath "$0"))
blender_ext_file="${scripts_dir}/blender_ext.py"
repo_root=$(dirname "${scripts_dir}")
wrap_dir="${repo_root}/rust/crates/wrap"
extension_dir="${repo_root}/python/src/squishy_volumes_extension"
wheels_dir="${extension_dir}/wheels"

rm -i "${wheels_dir}/*" || true

uv run --with toml "${scripts_dir}/update_lib_version.py"

"${scripts_dir}/add_license_headers.sh"

cd "${wrap_dir}"
uvx --python 3.11 maturin build --no-default-features --release --out "${wheels_dir}"

cd -
uv run --with toml "${scripts_dir}/update_manifest.py"

test -f "${blender_ext_file}" || wget https://raw.githubusercontent.com/blender/blender/refs/tags/v4.5.0/scripts/addons_core/bl_pkg/cli/blender_ext.py --output-document="${blender_ext_file}"

cd "${extension_dir}"
uv run --python 3.11 "${blender_ext_file}" build
uv run --python 3.11 "${blender_ext_file}" server-generate --repo-dir=./