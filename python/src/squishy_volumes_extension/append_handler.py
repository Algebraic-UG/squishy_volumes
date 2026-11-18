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

import bpy
from bpy.app.handlers import persistent

from .get_preferences import debug_log
from .registration import add_handler, remove_handler

# See also
# https://github.com/Algebraic-UG/squishy_volumes/issues/171


@persistent
def fix_appended_data(context):
    for item in context.import_items:
        obj = item.id
        if not isinstance(obj, bpy.types.Object):
            continue

        obj.squishy_volumes.uuid = ""


def register_append_handler():
    add_handler(bpy.app.handlers.blend_import_post, fix_appended_data)
    debug_log("append handler registered.")


def unregister_append_handler():
    remove_handler(bpy.app.handlers.blend_import_post, fix_appended_data)
    debug_log("append handler unregistered.")
