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

import json
import bpy

from ..squishy_volumes_properties import (
    get_simulation_object_with_uuid,
    get_selected_simulation_object,
    get_selected_simulation_uuid,
    get_input_objects_with_uuid,
    Squishy_Volumes_Properties_Simulation,
)
from ..bridge import SimulationHandle, SimulationInputHandle
from ..util import giga_f32_to_u64, simulation_input_exists
from ..input_capture import create_input_header, capture_input_frame
from ..preferences import get_confirm_bake_overwrite


def _start_compute(
    sim_handle: SimulationHandle,
    sim_props: Squishy_Volumes_Properties_Simulation,
    next_frame: int,
    number_of_frames: int,
):
    compute_settings = {
        "time_step": sim_props.time_step,
        "gpu": sim_props.gpu,
        "adaptive_time_steps": sim_props.adaptive_time_steps,
        "next_frame": next_frame,
        "number_of_frames": number_of_frames,
        "max_bytes_on_disk": giga_f32_to_u64(sim_props.max_giga_bytes_on_disk),
    }
    sim_handle.start_compute(compute_settings=compute_settings)


SIMULATION_INPUT = None


class SCENE_OT_Squishy_Volumes_Record_Input_To_Cache(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_record_input_to_cache"
    bl_label = "Record Input"
    bl_description = """(Over)Write the cache with the new input.

This writes global settings as well as object specific settings
to the simulation cache.

Note that this also discards all computed frames in the cache."""
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore
    blocking: bpy.props.BoolProperty(default=False)  # type: ignore
    start_baking: bpy.props.BoolProperty(default=False)  # type: ignore

    def execute(self, context: bpy.types.Context):
        assert context.scene is not None
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        sim_props.has_loaded_frame = False

        self.report({"INFO"}, f"Resetting {sim_obj.name}")

        sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
        if sim_handle is not None:
            sim_handle.drop()

        input_header = create_input_header(sim_props)

        self.report({"INFO"}, f"Collected input header for {sim_obj.name}")

        sim_input_handle = SimulationInputHandle.new(
            uuid=self.uuid,
            directory=sim_props.directory,
            input_header=input_header,
            max_bytes_on_disk=giga_f32_to_u64(sim_props.max_giga_bytes_on_disk),
        )

        self.report({"INFO"}, f"(Re)Created {sim_obj.name}")

        if not self.blocking:
            global SIMULATION_INPUT
            SIMULATION_INPUT = sim_input_handle
            bpy.ops.scene.squishy_volumes_record_input_to_cache_modal(  # ty:ignore[unresolved-attribute]
                "INVOKE_DEFAULT", uuid=self.uuid, start_baking=self.start_baking
            )
            return {"FINISHED"}

        prior_frame = context.scene.frame_current
        context.scene.frame_set(sim_props.capture_start_frame)

        for i in range(sim_props.capture_frames):
            capture_input_frame(
                sim_props=sim_props,
                sim_input_handle=sim_input_handle,
            )
            if i + 1 < sim_props.capture_frames:
                context.scene.frame_set(context.scene.frame_current + 1)

        context.scene.frame_set(prior_frame)

        sim_handle = SimulationHandle.new()
        if self.start_baking:
            _start_compute(sim_handle, sim_props, 0, sim_props.bake_frames)
            self.report({"INFO"}, f"Commence baking of {sim_obj.name}.")

        return {"FINISHED"}

    def invoke(self, context, event):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        if (
            simulation_input_exists(sim_obj.squishy_volumes.directory)  # ty:ignore[unresolved-attribute]
            and get_confirm_bake_overwrite()
            and not self.blocking  # implies script usage
        ):
            return context.window_manager.invoke_props_dialog(self)
        else:
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
    bl_options = set()

    uuid: bpy.props.StringProperty()  # type: ignore
    start_baking: bpy.props.BoolProperty(default=False)  # type: ignore

    _timer = None
    prior_frame = None

    def invoke(self, context, event):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

        self.prior_frame = context.scene.frame_current
        context.scene.frame_set(sim_props.capture_start_frame)

        self._timer = context.window_manager.event_timer_add(
            time_step=0, window=context.window
        )
        context.window_manager.progress_begin(0, sim_props.capture_frames)
        context.window_manager.modal_handler_add(self)

        return {"RUNNING_MODAL"}

    def modal(self, context, event):
        global SIMULATION_INPUT
        assert isinstance(SIMULATION_INPUT, SimulationInputHandle)
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

        if event.type in {"RIGHTMOUSE", "ESC"}:
            context.window_manager.event_timer_remove(self._timer)
            SIMULATION_INPUT.drop()
            self.report(
                {"WARNING"},
                f"Capture of {sim_obj.name} incomplete due to user cancellation.",
            )
            context.scene.frame_set(self.prior_frame)
            return {"CANCELLED"}

        if event.type != "TIMER":
            return {"RUNNING_MODAL"}

        captured_frames = context.scene.frame_current - sim_props.capture_start_frame
        assert captured_frames >= 0

        if captured_frames < sim_props.capture_frames:
            try:
                capture_input_frame(
                    sim_props=sim_props,
                    sim_input_handle=SIMULATION_INPUT,
                )
            except RuntimeError:
                SIMULATION_INPUT.drop()
                raise

            context.window_manager.progress_update(captured_frames)

        if captured_frames + 1 < sim_props.capture_frames:
            context.scene.frame_set(context.scene.frame_current + 1)
            return {"RUNNING_MODAL"}

        context.scene.frame_set(self.prior_frame)
        context.window_manager.progress_end()

        self.report({"INFO"}, f"Finished capturing input for {sim_obj.name}")

        SIMULATION_INPUT = None
        sim_handle = SimulationHandle.new()

        if self.start_baking:
            _start_compute(sim_handle, sim_props, 0, sim_props.bake_frames)
            self.report({"INFO"}, f"Commence baking of {sim_obj.name}.")

        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_bake_start_from_latest"
    bl_label = "Bake (from latest)"
    bl_description = """Continue baking physics.

This uses the latest state available and runs the simulation
either until the desired number of frames is reached
or cancellation occurs due to user input or error."""
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore

    @classmethod
    def poll(cls, context):
        sim_obj = get_selected_simulation_object(context.scene)
        if sim_obj is None:
            return False
        uuid = sim_obj.squishy_volumes.uuid  # ty:ignore[unresolved-attribute]
        sim_obj = get_simulation_object_with_uuid(uuid)
        sim_handle = SimulationHandle.get(uuid=uuid)
        return (
            sim_handle is not None
            and not sim_handle.computing()
            and sim_handle.available_frames() < sim_obj.squishy_volumes.bake_frames  # ty:ignore[unresolved-attribute]
        )

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        assert sim_handle is not None
        _start_compute(
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
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore

    @classmethod
    def poll(cls, context):
        sim_obj = get_selected_simulation_object(context.scene)
        if sim_obj is None:
            return False
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        if sim_obj is None or not sim_props.has_loaded_frame:
            return False
        sim_handle = SimulationHandle.get(uuid=sim_props.uuid)
        return (
            sim_handle is not None
            and not sim_handle.computing()
            and sim_props.loaded_frame + 1 < sim_props.bake_frames
        )

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        assert sim_handle is not None
        _start_compute(
            sim_handle=sim_handle,
            sim_props=sim_props,
            next_frame=sim_props.loaded_frame + 1,
            number_of_frames=sim_props.bake_frames,
        )
        self.report({"INFO"}, f"Commence baking of {sim_obj.name}.")
        return {"FINISHED"}


class SCENE_OT_Squishy_Volumes_Bake_Pause(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_bake_pause"
    bl_label = "Pause"
    bl_description = "Pause the computation of the simulation frames."
    bl_options = {"REGISTER"}

    uuid: bpy.props.StringProperty()  # type: ignore

    @classmethod
    def poll(cls, context):
        uuid = get_selected_simulation_uuid(context.scene)
        if uuid is None:
            return False
        sim_handle = SimulationHandle.get(uuid=uuid)
        return sim_handle is not None and sim_handle.computing()

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        assert sim_handle is not None
        sim_handle.pause_compute()
        self.report({"INFO"}, f"Baking of {sim_obj.name} paused.")
        return {"FINISHED"}


class SCENE_PT_Squishy_Volumes_Simulate(bpy.types.Panel):
    bl_label = "Simulate"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"
    bl_options = set()

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
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]

        record_box = self.layout.box()
        record_box.label(text="Record Input")

        frame_row = record_box.row()
        frame_row.prop(sim_props, "capture_start_frame")
        frame_row.prop(sim_props, "capture_frames")

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
        bake_box.prop(sim_props, "gpu")
        # TODO: make implicit viable
        # col.prop(simulation, "explicit")
        # col.prop(simulation, "debug_mode")

        # TODO: enable adaptive time steps on gpu
        adaptive_col = bake_box.column()
        adaptive_col.enabled = not sim_props.gpu
        adaptive_col.prop(sim_props, "adaptive_time_steps")

        bake_box.prop(sim_props, "bake_frames")

        row = bake_box.row()
        row.operator(
            SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest.bl_idname,
            icon="PHYSICS",
        ).uuid = sim_props.uuid
        if (
            sim_props.has_loaded_frame
            and sim_props.loaded_frame + 1 != sim_handle.available_frames()
        ):
            row.operator(
                SCENE_OT_Squishy_Volumes_Bake_Start_From_Loaded.bl_idname,
                text=f"Rebake from #{sim_props.loaded_frame}",
                icon="PHYSICS",
            ).uuid = sim_props.uuid

        bake_box.operator(
            SCENE_OT_Squishy_Volumes_Bake_Pause.bl_idname,
            icon="CANCEL",
        ).uuid = sim_props.uuid

        if sim_handle.progress is not None:
            for info in sim_handle.progress:
                name = info["label"]
                completed_steps = info["completed_steps"]
                steps_to_completion = info["steps_to_completion"]
                bake_box.progress(
                    text=f"{name}: {completed_steps}/{steps_to_completion}",
                    factor=completed_steps / steps_to_completion,
                )


classes = [
    SCENE_OT_Squishy_Volumes_Record_Input_To_Cache,
    SCENE_OT_Squishy_Volumes_Record_Input_To_Cache_Modal,
    SCENE_OT_Squishy_Volumes_Bake_Start_From_Latest,
    SCENE_OT_Squishy_Volumes_Bake_Start_From_Loaded,
    SCENE_OT_Squishy_Volumes_Bake_Pause,
    SCENE_PT_Squishy_Volumes_Simulate,
]


def register_panel_simulate():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_simulate():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
