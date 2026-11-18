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

from ..registration import add_handler, remove_handler
from ..util import index_by_object, obj_by_index
from .object import get_simulation_objects


def _poll_simulation(_, obj: bpy.types.Object) -> bool:
    return obj.squishy_volumes.is_simulation


def get_selected_simulation_object(scene: bpy.types.Scene) -> bpy.types.Object | None:
    sim_objs = get_simulation_objects()
    if len(sim_objs) == 1:
        return sim_objs[0]

    selected = scene.squishy_volumes.selected_simulation
    if selected is None or not selected.squishy_volumes.is_simulation:
        return None
    return selected


def get_selected_simulation_uuid(scene: bpy.types.Scene) -> str | None:
    sim_obj = get_selected_simulation_object(scene)
    if sim_obj is None:
        return None
    return sim_obj.squishy_volumes.uuid


def _verify_selected_object(
    obj: bpy.types.Object, scene: bpy.types.Scene
) -> bpy.types.Object | None:
    uuid = get_selected_simulation_uuid(scene)
    if uuid is None or obj.squishy_volumes.uuid != uuid:
        return None
    return obj


def get_selected_input_object(scene: bpy.types.Scene) -> bpy.types.Object | None:
    obj = obj_by_index(scene.squishy_volumes.selected_input_object)
    if obj is None or obj.type == "EMPTY":
        return None
    return _verify_selected_object(obj, scene)


def _update_selection(index, context):
    obj = obj_by_index(index)
    if obj is None:
        return

    if obj.name in context.view_layer.objects:
        context.view_layer.objects.active = obj
        obj.select_set(True)


def _could_be_selected(obj: bpy.types.Object) -> bool:
    return obj.visible_get() and not obj.hide_select


def _list_index_for_viewport_selection() -> int:
    active = bpy.context.view_layer.objects.active
    if active is None or active.squishy_volumes.is_simulation:
        return -1
    if active.select_get() or not _could_be_selected(active):
        return index_by_object(active)
    return -1


@persistent
def _sync_with_viewport_selection(scene, _depsgraph):
    index = _list_index_for_viewport_selection()
    if scene.squishy_volumes.selected_input_object != index:
        scene.squishy_volumes.selected_input_object = index


def subscribe_to_selection():
    add_handler(bpy.app.handlers.depsgraph_update_post, _sync_with_viewport_selection)


def unsubscribe_from_selection():
    remove_handler(
        bpy.app.handlers.depsgraph_update_post, _sync_with_viewport_selection
    )


class Squishy_Volumes_Properties_Scene(bpy.types.PropertyGroup):
    selected_simulation: bpy.props.PointerProperty(
        type=bpy.types.Object,
        poll=_poll_simulation,
        name="Selected Simulation",
        description="Most operations assume this simulation as the target.",
        options=set(),
    )
    selected_input_object: bpy.props.IntProperty(
        name="Selected Input Object",
        description="The selected input object can have it's setting edited or removed.",
        update=lambda self, context: _update_selection(
            self.selected_input_object, context
        ),
        options=set(),
    )
