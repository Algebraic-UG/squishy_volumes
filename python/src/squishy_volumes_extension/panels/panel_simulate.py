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

from ..bridge import SimulationHandle
from ..capture_markers import capture_markers, capture_markers_error
from ..get_preferences import (
    first_run_setup_pending,
    get_enable_timeline_markers,
    get_preferences,
)
from ..operators.simulate import (
    SCENE_OT_Squishy_Volumes_Bake_Pause,
    SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest,
    SCENE_OT_Squishy_Volumes_Bake_Start_From_Loaded,
    SCENE_OT_Squishy_Volumes_Fix_Capture_Markers,
    SCENE_OT_Squishy_Volumes_Record_Input_To_Cache,
    rebake_frame,
)
from ..properties import (
    Squishy_Volumes_Properties_Simulation,
    capture_frame_count,
    get_input_objects_with_uuid,
    get_selected_simulation_object,
    get_selected_simulation_uuid,
)
from ..registration import class_registration


def draw_capture_range(
    layout: bpy.types.UILayout, scene: bpy.types.Scene, sim_obj: bpy.types.Object
):
    if get_enable_timeline_markers():
        draw_capture_markers(layout, scene, sim_obj)
    else:
        draw_capture_properties(layout, sim_obj.squishy_volumes)


def draw_capture_properties(
    layout: bpy.types.UILayout, sim_props: Squishy_Volumes_Properties_Simulation
):
    layout.label(text=f"Record Input: {capture_frame_count(sim_props)} frames")
    frame_row = layout.row()
    frame_row.prop(sim_props, "capture_start_frame")
    frame_row.prop(sim_props, "capture_end_frame")


def draw_capture_markers(
    layout: bpy.types.UILayout, scene: bpy.types.Scene, sim_obj: bpy.types.Object
):
    markers = capture_markers(scene, sim_obj)
    if len(markers) != 2:
        layout.label(text="Record Input")
        layout.label(text=capture_markers_error(scene, sim_obj), icon="ERROR")
        layout.operator(
            SCENE_OT_Squishy_Volumes_Fix_Capture_Markers.bl_idname,
            icon="MARKER_HLT",
        ).uuid = sim_obj.squishy_volumes.uuid
        return

    start, end = markers
    layout.label(
        text=f"Record Input: {end.frame - start.frame + 1} frames, set by markers",
        icon="MARKER_HLT",
    )
    frame_row = layout.row()
    frame_row.prop(start, "frame", text="Start")
    frame_row.prop(end, "frame", text="End")


class SCENE_PT_Squishy_Volumes_Simulate(bpy.types.Panel):
    bl_label = "Simulate"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"
    bl_options = set()  # noqa: RUF012

    @classmethod
    def poll(cls, context):
        if context.mode != "OBJECT" or first_run_setup_pending(get_preferences()):
            return False
        uuid = get_selected_simulation_uuid(context.scene)
        return uuid is not None and (
            SimulationHandle.exists(uuid=uuid) or get_input_objects_with_uuid(uuid)
        )

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        sim_obj = get_selected_simulation_object(context.scene)
        assert sim_obj is not None
        sim_props = sim_obj.squishy_volumes

        record_box = self.layout.box()
        draw_capture_range(record_box, context.scene, sim_obj)

        record_op = record_box.operator(
            SCENE_OT_Squishy_Volumes_Record_Input_To_Cache.bl_idname,
            icon="FILE_CACHE",
            text=SCENE_OT_Squishy_Volumes_Record_Input_To_Cache.bl_label + " & Pause",
        )
        record_op.uuid = sim_props.uuid
        record_op.start_baking = False

        self.layout.separator()

        record_and_bake_op = self.layout.operator(
            SCENE_OT_Squishy_Volumes_Record_Input_To_Cache.bl_idname,
            icon="PHYSICS",
            text=SCENE_OT_Squishy_Volumes_Record_Input_To_Cache.bl_label
            + " & Bake Simulation",
        )
        record_and_bake_op.uuid = sim_props.uuid
        record_and_bake_op.start_baking = True

        self.layout.separator()

        sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
        if sim_handle is None:
            return

        bake_box = self.layout.box()
        bake_box.label(text="Bake Simulation")

        bake_box.prop(sim_props, "time_step")
        bake_box.prop(sim_props, "compute_device")
        bake_box.prop(sim_props, "adaptive_time_steps")

        bake_box.prop(sim_props, "bake_frames")

        row = bake_box.row()
        row.operator(
            SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest.bl_idname,
            icon="PHYSICS",
        ).uuid = sim_props.uuid
        loaded_frame = rebake_frame(sim_handle, sim_props, context.scene)
        if loaded_frame is not None:
            row.operator(
                SCENE_OT_Squishy_Volumes_Bake_Start_From_Loaded.bl_idname,
                text=f"Rebake from #{loaded_frame}",
                icon="PHYSICS",
            ).uuid = sim_props.uuid

        bake_box.operator(
            SCENE_OT_Squishy_Volumes_Bake_Pause.bl_idname,
            icon="CANCEL",
        ).uuid = sim_props.uuid

        if sim_handle.poll_info is not None:
            for info in sim_handle.poll_info["progress"]:
                name = info["label"]
                completed_steps = info["completed_steps"]
                steps_to_completion = info["steps_to_completion"]
                bake_box.progress(
                    text=f"{name}: {completed_steps}/{steps_to_completion}",
                    factor=completed_steps / steps_to_completion,
                )


classes = [
    SCENE_PT_Squishy_Volumes_Simulate,
]


register_panel_simulate, unregister_panel_simulate = class_registration(classes)
