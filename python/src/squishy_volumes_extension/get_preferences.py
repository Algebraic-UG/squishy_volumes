# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
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

import bpy
import mathutils


def get_confirm_bake_overwrite() -> bool:
    return bpy.context.preferences.addons.get(
        __package__
    ).preferences.confirm_bake_overwrite


def get_domain_min() -> mathutils.Vector:
    return bpy.context.preferences.addons.get(__package__).preferences.domain_min


def get_domain_max() -> mathutils.Vector:
    return bpy.context.preferences.addons.get(__package__).preferences.domain_max


def get_print_debug_info() -> bool:
    return bpy.context.preferences.addons.get(__package__).preferences.print_debug_info
