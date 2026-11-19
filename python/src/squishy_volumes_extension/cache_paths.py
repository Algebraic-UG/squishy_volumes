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

from pathlib import Path

import bpy

BLEND_RELATIVE_PREFIX = "//"
SIMULATION_INPUT_FILE = "simulation_input.bin"
STATS_FILE = "stats.csv"
FIRST_FRAME_FILE = "frame_00000.abc"


def _join_keeping_blend_relative_prefix(directory: str, *parts: str) -> str:
    if directory.startswith(BLEND_RELATIVE_PREFIX):
        relative = directory.removeprefix(BLEND_RELATIVE_PREFIX)
        return BLEND_RELATIVE_PREFIX + str(Path(relative, *parts))
    return str(Path(directory, *parts))


def normalize_directory(directory: str) -> str:
    return _join_keeping_blend_relative_prefix(directory)


def first_frame_file(directory: str) -> str:
    return _join_keeping_blend_relative_prefix(directory, FIRST_FRAME_FILE)


def cache_directory_is_resolvable(directory: str) -> bool:
    return not directory.startswith(BLEND_RELATIVE_PREFIX) or bool(bpy.data.filepath)


def absolute_cache_directory(directory: str) -> Path:
    if not cache_directory_is_resolvable(directory):
        raise RuntimeError(
            f"The cache directory '{directory}' is relative to the .blend file.\n"
            "Save the .blend file first or choose an absolute cache directory."
        )
    return Path(bpy.path.abspath(directory))


def simulation_input_exists(directory: str) -> bool:
    return (
        cache_directory_is_resolvable(directory)
        and (absolute_cache_directory(directory) / SIMULATION_INPUT_FILE).exists()
    )


# written by the store thread, see rust/crates/cache/src/stats.rs
def simulation_stats_path(directory: str) -> Path:
    return absolute_cache_directory(directory) / STATS_FILE
