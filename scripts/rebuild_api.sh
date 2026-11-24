#!/bin/bash

set -e

cd "$(dirname "$0")/.."

rm -f python/src/squishy_volumes_extension/wheels/*.whl
rm -f rust/target/{debug,release}/libsquishy_volumes_hot-hot-*

echo "Updating lib version"
uv run --with toml scripts/update_lib_version.py

echo "Adding license headers"
./scripts/add_license_headers.sh

cd rust/crates/wrap
uvx --python 3.11 maturin build --release --out ../../../python/src/squishy_volumes_extension/wheels/

cd -
cd rust/crates/hot
cargo build --release

echo "Updating manifest"
cd -
uv run --with toml scripts/update_manifest.py
