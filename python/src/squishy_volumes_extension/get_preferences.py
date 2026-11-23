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

import os

import bpy
import mathutils

DEFAULT_DEVICE_VARIABLE = "SQUISHY_VOLUMES_DEFAULT_DEVICE"
DEFAULT_CACHE_VARIABLE = "SQUISHY_VOLUMES_DEFAULT_CACHE"


def get_preferences():
    return bpy.context.preferences.addons.get(__package__).preferences


def get_default_cache_location() -> str:
    return get_preferences().default_cache_location


def first_run_setup_pending(preferences, environment=os.environ) -> bool:
    if preferences.first_run_setup_done:
        return False
    return not (
        DEFAULT_DEVICE_VARIABLE in environment and DEFAULT_CACHE_VARIABLE in environment
    )


def apply_environment_defaults(preferences, environment=os.environ):
    if DEFAULT_DEVICE_VARIABLE in environment:
        preferences.default_compute_device_identifier = environment[
            DEFAULT_DEVICE_VARIABLE
        ]
    if DEFAULT_CACHE_VARIABLE in environment:
        preferences.default_cache_location = environment[DEFAULT_CACHE_VARIABLE]


def get_default_compute_device() -> str:
    return get_preferences().default_compute_device


def get_sanity_check_allowed_disk_space() -> bool:
    return get_preferences().sanity_check_allowed_disk_space


def get_confirm_bake_overwrite() -> bool:
    return get_preferences().confirm_bake_overwrite


def get_domain_min() -> mathutils.Vector:
    return get_preferences().domain_min


def get_domain_max() -> mathutils.Vector:
    return get_preferences().domain_max


def get_print_debug_info() -> bool:
    return get_preferences().print_debug_info


def debug_log(message: str):
    if get_print_debug_info():
        print(f"Squishy Volumes {message}")


def get_enable_timeline_markers() -> bool:
    return get_preferences().enable_timeline_markers


def get_allow_scaled_input() -> bool:
    return get_preferences().allow_scaled_input
