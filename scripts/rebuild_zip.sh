#!/bin/bash

set -e

cd "$(dirname "$0")/.."

rm -f python/wheels/*

uv run --with toml scripts/update_lib_version.py

./scripts/add_license_headers.sh

cd rust/crates/wrap
uvx --python 3.11 maturin build --no-default-features --release --out ../../../python/wheels/

cd -
uv run --with toml scripts/update_manifest.py

test -f ./scripts/blender_ext.py || wget https://raw.githubusercontent.com/blender/blender/refs/tags/v4.5.0/scripts/addons_core/bl_pkg/cli/blender_ext.py --output-document=./scripts/blender_ext.py

cd python
uv run --python 3.11 ../scripts/blender_ext.py build