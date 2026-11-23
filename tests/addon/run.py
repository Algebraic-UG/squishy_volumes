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

# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///

import argparse
import os
import subprocess
import sys
import tempfile
from pathlib import Path

TESTS_DIR = Path(__file__).resolve().parent


def parse_arguments():
    parser = argparse.ArgumentParser(
        description="Run the addon tests in headless Blender "
        "against a built extension zip, using a throwaway Blender user directory."
    )
    parser.add_argument(
        "--zip",
        required=True,
        type=Path,
        help="extension zip, for example built by scripts/rebuild_zip.sh",
    )
    parser.add_argument(
        "--blender",
        default=os.environ.get("BLENDER", "blender"),
        help="Blender executable (default: $BLENDER or blender)",
    )
    parser.add_argument(
        "patterns",
        nargs="*",
        help="only run tests whose name contains one of these patterns",
    )
    return parser.parse_args()


def run_in_blender(blender: str, zip_path: Path, patterns: list[str]) -> int:
    with tempfile.TemporaryDirectory(prefix="squishy_volumes_user_") as user_dir:
        command = [
            blender,
            "--background",
            "--factory-startup",
            "--python-exit-code",
            "1",
            "--python",
            str(TESTS_DIR / "in_blender.py"),
            "--",
            str(zip_path.resolve()),
            *patterns,
        ]
        environment = {
            name: value
            for name, value in os.environ.items()
            if not name.startswith("SQUISHY_VOLUMES_")
        }
        environment["BLENDER_USER_RESOURCES"] = user_dir
        return subprocess.run(command, env=environment, check=False).returncode


if __name__ == "__main__":
    arguments = parse_arguments()
    sys.exit(run_in_blender(arguments.blender, arguments.zip, arguments.patterns))
