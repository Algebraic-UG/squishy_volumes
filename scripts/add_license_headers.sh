#!/usr/bin/env bash

set -euo pipefail

cd "$(dirname "$0")/.."

# Define the headers
PY_HEADER=$(cat << EOF
# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Blended MPM extension.
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
EOF
)

RS_HEADER=$(cat << EOF
// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.
EOF
)

# Function to check if a header already exists
has_header() {
    grep -q "SPDX-License-Identifier" "$1"
}

# Add headers to Python files
cd python
echo "checking python files"
git ls-files "*.py" | while read -r file; do
    echo "checking $file"
    if ! has_header "$file"; then
        echo "Adding header to $file"
        tmp_file=$(mktemp)
        echo "$PY_HEADER" > "$tmp_file"
        echo "" >> "$tmp_file"
        cat "$file" >> "$tmp_file"
        mv "$tmp_file" "$file"
    fi
done
cd -

# Add headers to Rust files
echo "checking rust files"
cd rust
git ls-files "*.rs" | while read -r file; do
    echo "checking $file"
    if ! has_header "$file"; then
        echo "Adding header to $file"
        tmp_file=$(mktemp)
        echo "$RS_HEADER" > "$tmp_file"
        echo "" >> "$tmp_file"
        cat "$file" >> "$tmp_file"
        mv "$tmp_file" "$file"
    fi
done
cd -

echo "✅ Headers added where missing."
