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

import shutil
from collections.abc import Generator

import bpy

from .. import api
from ..api.record import release_cache_file
from ..bridge import SimulationHandle
from ..cache_paths import absolute_cache_directory, simulation_input_exists
from ..capture_markers import (
    capture_markers,
    capture_markers_error,
    place_capture_markers,
    range_to_record,
    remove_superfluous_capture_markers,
    superfluous_capture_markers,
)
from ..get_preferences import (
    get_confirm_bake_overwrite,
    get_enable_timeline_markers,
    get_sanity_check_allowed_disk_space,
)
from ..operator_base import SimulationOperator, report_error
from ..registration import class_registration
from ..squishy_volumes_properties import (
    Squishy_Volumes_Properties_Simulation,
    capture_frame_count,
    frame_to_load,
    get_input_objects_with_uuid,
    get_selected_simulation_object,
    get_selected_simulation_uuid,
    get_simulation_object_with_uuid,
    stored_range,
)
from ..util import (
    giga_f32_to_u64,
    u64_to_giga_f32,
)


def rebake_frame(
    sim_handle: SimulationHandle,
    sim_props: Squishy_Volumes_Properties_Simulation,
    scene: bpy.types.Scene,
) -> int | None:
    """The displayed frame, if a rebake from it would replace baked frames."""
    loaded_frame = frame_to_load(sim_props, scene.frame_current)
    if loaded_frame is None or loaded_frame + 1 >= sim_handle.available_frames():
        return None
    return loaded_frame


def start_compute(
    sim_handle: SimulationHandle,
    sim_props: Squishy_Volumes_Properties_Simulation,
    next_frame: int,
    number_of_frames: int,
) -> bool:
    sim_handle.last_error = None

    if get_sanity_check_allowed_disk_space():
        allowed = sim_props.max_giga_bytes_on_disk
        free = u64_to_giga_f32(
            shutil.disk_usage(absolute_cache_directory(sim_props.directory)).free
        )
        if allowed > free:
            raise RuntimeError(
                f"""The disk might run out of space!

Squishy Volumes is allowed to use up to {allowed:.2f} GB,
but only {free:.2f} GB are free.

This check can be disabled in the add-on preferences."""
            )

    compute_settings = {
        "time_step": sim_props.time_step,
        "gpu": None if sim_props.compute_device == "CPU" else sim_props.compute_device,
        "adaptive_time_steps": sim_props.adaptive_time_steps,
        "next_frame": next_frame,
        "number_of_frames": number_of_frames,
        "max_bytes_on_disk": giga_f32_to_u64(sim_props.max_giga_bytes_on_disk),
    }
    release_cache_file(sim_props)
    sim_handle.start_compute(compute_settings=compute_settings)

    return True


class SCENE_OT_Squishy_Volumes_Fix_Capture_Markers(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_fix_capture_markers"
    bl_label = "Fix Timeline Markers"
    bl_description = """Each simulation needs exactly two timeline markers
named like the simulation, marking the first and last frame to record.

With fewer, both are restored at the previously recorded range.
With more, only the earliest and the latest are kept."""
    bl_options = {"REGISTER", "UNDO"}  # noqa: RUF012

    uuid: bpy.props.StringProperty()

    @classmethod
    def poll(cls, context):
        return get_enable_timeline_markers()

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        if len(capture_markers(context.scene, sim_obj)) < 2:
            start, end = stored_range(sim_obj.squishy_volumes)
            place_capture_markers(context.scene, sim_obj, start, end)
        else:
            remove_superfluous_capture_markers(context.scene, sim_obj)
        self.report({"INFO"}, f"Fixed timeline markers of {sim_obj.name}.")
        return {"FINISHED"}

    def invoke(self, context, event):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        restoring = len(capture_markers(context.scene, sim_obj)) < 2
        return context.window_manager.invoke_props_dialog(
            self,
            title=f"Squishy Volumes: {sim_obj.name}",
            confirm_text="Restore Markers" if restoring else "Delete Superfluous",
            width=400,
        )

    def draw(self, context):
        assert self.layout is not None
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        markers = capture_markers(context.scene, sim_obj)
        self.layout.label(text=capture_markers_error(context.scene, sim_obj) + ".")
        if len(markers) < 2:
            start, end = stored_range(sim_obj.squishy_volumes)
            self.layout.label(text=f"Restore them at frames {start} and {end}?")
        else:
            superfluous = superfluous_capture_markers(context.scene, sim_obj)
            frames = ", ".join(str(marker.frame) for marker in superfluous)
            self.layout.label(
                text=f"Keep frames {markers[0].frame} and {markers[-1].frame}?"
            )
            self.layout.label(text=f"Deletes the markers at frames {frames}.")


class SCENE_OT_Squishy_Volumes_Record_Input_To_Cache(
    SimulationOperator, bpy.types.Operator
):
    bl_idname = "scene.squishy_volumes_record_input_to_cache"
    bl_label = "Record Input"
    bl_description = """(Over)Write the cache with the new input.

The input objects are evaluated from the capture start to the capture end frame.
With timeline markers enabled in the preferences,
these are set by the two markers named like the simulation.

This writes global settings as well as object specific settings
to the simulation cache.

Note that this also discards all computed frames in the cache."""
    bl_options = {"REGISTER"}  # noqa: RUF012

    start_baking: bpy.props.BoolProperty(default=False)

    def run(self, context):
        sim_obj = self.sim_obj
        api.capture_range(sim_obj, scene=context.scene)
        if not bpy.app.background:
            bpy.ops.scene.squishy_volumes_record_input_to_cache_modal(
                "INVOKE_DEFAULT", uuid=self.uuid, start_baking=self.start_baking
            )
            return {"FINISHED"}
        recorded_frames = api.record_input(sim_obj, scene=context.scene)
        self.report({"INFO"}, f"Recorded {recorded_frames} frames of {sim_obj.name}.")
        if self.start_baking:
            start_baking(self, sim_obj)
        return {"FINISHED"}

    def invoke(self, context, event):
        sim_obj = self.sim_obj
        if range_to_record(context.scene, sim_obj) is None:
            bpy.ops.scene.squishy_volumes_fix_capture_markers(
                "INVOKE_DEFAULT", uuid=self.uuid
            )
            return {"CANCELLED"}
        if (
            simulation_input_exists(sim_obj.squishy_volumes.directory)
            and get_confirm_bake_overwrite()
        ):
            return context.window_manager.invoke_props_dialog(self)
        return self.execute(context)

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        if sim_handle is None:
            prior_frames = 0
        else:
            prior_frames = sim_handle.available_frames()
        self.layout.label(text="WARNING: This is a destructive operation!")
        self.layout.label(
            text=f"The previously record will be overwritten, including: {prior_frames} frames"
        )


class SCENE_OT_Squishy_Volumes_Record_Input_To_Cache_Modal(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_record_input_to_cache_modal"
    bl_label = "Record Input Modal"
    bl_options = set()  # noqa: RUF012

    uuid: bpy.props.StringProperty()
    start_baking: bpy.props.BoolProperty(default=False)

    _timer = None
    _recording = None
    _animation_was_playing = False

    def invoke(self, context, event):
        sim_obj = api.find_simulation(self.uuid)
        start, end = api.capture_range(sim_obj, scene=context.scene)
        self._animation_was_playing = bool(
            context.screen and context.screen.is_animation_playing
        )
        bpy.ops.screen.animation_cancel(restore_frame=False)
        self._recording = api.iter_record_input(sim_obj, scene=context.scene)
        self._timer = context.window_manager.event_timer_add(
            time_step=0, window=context.window
        )
        context.window_manager.progress_begin(0, end - start + 1)
        context.window_manager.modal_handler_add(self)
        return {"RUNNING_MODAL"}

    @property
    def recording(self) -> Generator[int]:
        assert self._recording is not None
        return self._recording

    def modal(self, context, event):
        if event.type in {"RIGHTMOUSE", "ESC"}:
            self.cancel(context)
            self.report({"WARNING"}, "Recording cancelled.")
            return {"CANCELLED"}
        if event.type != "TIMER":
            return {"RUNNING_MODAL"}
        finished = True
        try:
            recorded_frames = next(self.recording, None)
            if recorded_frames is not None:
                context.window_manager.progress_update(recorded_frames)
                finished = False
                return {"RUNNING_MODAL"}
        except RuntimeError as error:
            report_error(self, self.uuid, error)
            return {"CANCELLED"}
        finally:
            if finished:
                self.cancel(context)
        self._report_finished()
        return {"FINISHED"}

    def cancel(self, context):
        self.recording.close()
        if self._timer is not None:
            context.window_manager.event_timer_remove(self._timer)
            self._timer = None
            context.window_manager.progress_end()
        if self._animation_was_playing:
            self._animation_was_playing = False
            bpy.ops.screen.animation_play()

    def _report_finished(self):
        sim_obj = api.find_simulation(self.uuid)
        self.report({"INFO"}, f"Finished recording {sim_obj.name}.")
        if self.start_baking:
            try:
                start_baking(self, sim_obj)
            except RuntimeError as error:
                report_error(self, self.uuid, error)


def start_baking(operator, sim_obj: bpy.types.Object):
    sim_props = sim_obj.squishy_volumes
    sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
    assert sim_handle is not None
    start_compute(sim_handle, sim_props, 0, sim_props.bake_frames)
    operator.report({"INFO"}, f"Commence baking of {sim_obj.name}.")


class SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_bake_start_from_latest"
    bl_label = "Bake (from latest)"
    bl_description = """Continue baking physics.

This uses the latest state available and runs the simulation
either until the desired number of frames is reached
or cancellation occurs due to user input or error."""
    bl_options = {"REGISTER"}  # noqa: RUF012

    uuid: bpy.props.StringProperty()

    @classmethod
    def poll(cls, context):
        sim_obj = get_selected_simulation_object(context.scene)
        if sim_obj is None:
            return False
        uuid = sim_obj.squishy_volumes.uuid
        sim_obj = get_simulation_object_with_uuid(uuid)
        sim_handle = SimulationHandle.get(uuid=uuid)
        return (
            sim_handle is not None
            and not sim_handle.baking()
            and sim_handle.available_frames() < sim_obj.squishy_volumes.bake_frames
        )

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        assert sim_handle is not None
        start_compute(
            sim_handle=sim_handle,
            sim_props=sim_props,
            next_frame=sim_handle.available_frames(),
            number_of_frames=sim_props.bake_frames,
        )

        self.report({"INFO"}, f"Commence baking of {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Bake_Start_From_Loaded(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_bake_start_from_loaded"
    bl_label = "Bake"
    bl_description = """Restart baking physics.

This uses the displayed state and runs the simulation
either until the desired number of frames is reached
or cancellation occurs due to user input or error.

Note that this discards already computed frames that
come after the displayed one."""
    bl_options = {"REGISTER"}  # noqa: RUF012

    uuid: bpy.props.StringProperty()

    @classmethod
    def poll(cls, context):
        sim_obj = get_selected_simulation_object(context.scene)
        if sim_obj is None:
            return False
        sim_props = sim_obj.squishy_volumes
        if sim_obj is None:
            return False
        sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
        return (
            sim_handle is not None
            and not sim_handle.baking()
            and rebake_frame(sim_handle, sim_props, context.scene) is not None
        )

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        assert sim_handle is not None
        loaded_frame = rebake_frame(sim_handle, sim_props, context.scene)
        assert loaded_frame is not None
        start_compute(
            sim_handle=sim_handle,
            sim_props=sim_props,
            next_frame=loaded_frame + 1,
            number_of_frames=sim_props.bake_frames,
        )
        self.report({"INFO"}, f"Commence baking of {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Bake_Pause(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_bake_pause"
    bl_label = "Pause"
    bl_description = "Pause the computation of the simulation frames."
    bl_options = {"REGISTER"}  # noqa: RUF012

    uuid: bpy.props.StringProperty()

    @classmethod
    def poll(cls, context):
        uuid = get_selected_simulation_uuid(context.scene)
        if uuid is None:
            return False
        sim_handle = SimulationHandle.get(uuid=uuid)
        return sim_handle is not None and sim_handle.baking()

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        assert sim_handle is not None
        sim_handle.pause_compute()
        self.report({"INFO"}, f"Baking of {sim_obj.name} paused.")
        return {"FINISHED"}


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
        if context.mode != "OBJECT":
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
    SCENE_OT_Squishy_Volumes_Fix_Capture_Markers,
    SCENE_OT_Squishy_Volumes_Record_Input_To_Cache,
    SCENE_OT_Squishy_Volumes_Record_Input_To_Cache_Modal,
    SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest,
    SCENE_OT_Squishy_Volumes_Bake_Start_From_Loaded,
    SCENE_OT_Squishy_Volumes_Bake_Pause,
    SCENE_PT_Squishy_Volumes_Simulate,
]


register_panel_simulate, unregister_panel_simulate = class_registration(classes)
