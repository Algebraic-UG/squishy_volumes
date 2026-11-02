#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

# Define the headers
PY_HEADER=$(cat << EOF
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
EOF
)

RS_HEADER=$(echo "$PY_HEADER" | sed 's|^#|//|')

# Replace an existing header (everything up to the first empty line),
# or prepend the header if the file has none.
apply_header() {
    local file="$1"
    local header="$2"
    local tmp_file
    tmp_file=$(mktemp)
    echo "$header" > "$tmp_file"
    echo "" >> "$tmp_file"
    if head -n 1 "$file" | grep -q "SPDX-License-Identifier"; then
        awk 'in_body { print; next } /^$/ { in_body = 1 }' "$file" >> "$tmp_file"
    else
        cat "$file" >> "$tmp_file"
    fi
    if ! cmp -s "$tmp_file" "$file"; then
        echo "Updating header of $file"
        cat "$tmp_file" > "$file"
    fi
    rm "$tmp_file"
}

echo "checking python files"
git ls-files "python/*.py" "tests/*.py" "scripts/*.py" | while read -r file; do
    apply_header "$file" "$PY_HEADER"
done

echo "checking rust files"
cd rust
git ls-files "*.rs" | while read -r file; do
    apply_header "$file" "$RS_HEADER"
done
cd -

echo "checking shader files"
cd rust
git ls-files "*.wesl" "*.wgsl" | while read -r file; do
    # Skip files with an "external" directory component.
    if [[ "/$file/" == */external/* ]]; then
        echo "external: $file"
        continue
    fi
    apply_header "$file" "$RS_HEADER"
done
cd -

echo "✅ Headers up to date."
