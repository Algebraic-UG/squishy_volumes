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

from bpy.app.handlers import persistent

from .properties.squishy_volumes_object import IO_NONE
from .preferences import get_print_debug_info

# See also
# https://github.com/Algebraic-UG/squishy_volumes/issues/171


@persistent
def fix_appended_data(context):
    for item in context.import_items:
        obj = item.id
        if not isinstance(obj, bpy.types.Object):
            continue

        obj.squishy_volumes_object.simulation_uuid = "unassigned"
        obj.squishy_volumes_object.io = IO_NONE


def register_append_handler():
    if fix_appended_data not in bpy.app.handlers.blend_import_post:
        bpy.app.handlers.blend_import_post.append(fix_appended_data)
    if get_print_debug_info():
        print("Squishy Volumes append handler registered.")


def unregister_append_handler():
    if fix_appended_data in bpy.app.handlers.blend_import_post:
        bpy.app.handlers.blend_import_post.remove(fix_appended_data)
    if get_print_debug_info():
        print("Squishy Volumes append handler unregistered.")
