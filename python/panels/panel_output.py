# SPDX-License-Identifier: GPL-3.0-or-later
#
# This file is part of the Blended MPM extension.
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

from ..util import tutorial_msg

from ..nodes.drivers import remove_drivers
from ..magic_consts import (
    COLLIDER_MESH,
    COLLIDER_SAMPLES,
    FLUID_PARTICLES,
    GRID_COLLIDER_DISTANCE,
    GRID_MOMENTUM_CONFORMED,
    GRID_MOMENTUM_FREE,
    INPUT_MESH,
    SOLID_PARTICLES,
)

from ..properties.blended_mpm_object_attributes import Blended_MPM_Optional_Attributes
from ..output import (
    create_output,
    sync_output,
)
from ..properties.util import (
    get_output_objects,
    get_selected_output_object,
    get_selected_simulation,
)
from ..bridge import InputNames, available_frames, context_exists


def draw_object_attributes(layout, output_type, optional_attributes):
    if output_type == COLLIDER_MESH:
        return
    if output_type == INPUT_MESH:
        return

    layout.label(text="Please mouse-over for the exact identifier.")
    grid = layout.grid_flow(row_major=True, columns=2, even_columns=False)
    grid.label(text="Attribute")
    grid.label(text="Type")
    if output_type == GRID_COLLIDER_DISTANCE:
        grid.prop(optional_attributes, "grid_collider_distances")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "grid_collider_normal")
        grid.label(text="FLOAT_VECTOR")
    if output_type == GRID_MOMENTUM_FREE:
        grid.prop(optional_attributes, "grid_momentum_masses")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "grid_momentum_velocities")
        grid.label(text="FLOAT_VECTOR")
    if output_type == GRID_MOMENTUM_CONFORMED:
        grid.prop(optional_attributes, "grid_momentum_masses")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "grid_momentum_velocities")
        grid.label(text="FLOAT_VECTOR")
    if output_type == SOLID_PARTICLES:
        grid.prop(optional_attributes, "solid_masses")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "solid_initial_volumes")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "solid_velocities")
        grid.label(text="FLOAT_VECTOR")
        grid.prop(optional_attributes, "solid_transformations")
        grid.label(text="FLOAT4X4")
        grid.prop(optional_attributes, "solid_energies")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "solid_collider_insides")
        grid.label(text="FLOAT")
    if output_type == FLUID_PARTICLES:
        grid.prop(optional_attributes, "fluid_velocities")
        grid.label(text="FLOAT_VECTOR")
        grid.prop(optional_attributes, "fluid_transformations")
        grid.label(text="FLOAT4X4")
        grid.prop(optional_attributes, "fluid_collider_insides")
        grid.label(text="FLOAT")
        grid.prop(optional_attributes, "fluid_pressures")
        grid.label(text="FLOAT")
    if output_type == COLLIDER_SAMPLES:
        grid.prop(optional_attributes, "collider_normals")
        grid.label(text="FLOAT_VECTOR")
        grid.prop(optional_attributes, "collider_velocities")
        grid.label(text="FLOAT_VECTOR")


class OBJECT_OT_Blended_MPM_Jump_To_Start(bpy.types.Operator):
    bl_idname = "object.blended_mpm_jump_to_start"
    bl_label = "Jump to First Frame"
    bl_description = """Jump to the first frame that is available
in the loaded cache."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        simulation = get_selected_simulation(context)
        context.scene.frame_set(simulation.display_start_frame)

        self.report(
            {"INFO"},
            f"Jumped to first frame of {simulation.name}.",
        )
        return {"FINISHED"}


class OBJECT_OT_Blended_MPM_Add_Output_Object(bpy.types.Operator):
    bl_idname = "object.blended_mpm_add_output_object"
    bl_label = "Add Output Object"
    bl_description = """Create a new active output object from the simulation cache.

There are several output object types.
Most outputs are point-based and the object will be populated
with certain attributes and geometry node modifier.

As long as the object is an active output
and the current frame is availbe in the cache,
the positions and attributes are synchronized
each frame."""
    bl_options = {"REGISTER", "UNDO"}

    object_name: bpy.props.StringProperty()  # type: ignore
    output_type: bpy.props.StringProperty()  # type: ignore
    input_name: bpy.props.StringProperty()  # type: ignore
    num_colliders: bpy.props.IntProperty()  # type: ignore
    optional_attributes: bpy.props.PointerProperty(type=Blended_MPM_Optional_Attributes)  # type: ignore

    def execute(self, context):
        simulation = get_selected_simulation(context)

        mesh_name = f"{self.output_type} - {self.object_name}"
        obj = bpy.data.objects.new(self.object_name, bpy.data.meshes.new(mesh_name))

        obj.blended_mpm_object.input_name = self.input_name
        obj.blended_mpm_object.simulation_uuid = simulation.uuid
        obj.blended_mpm_object.output_type = self.output_type
        obj.blended_mpm_object.attributes = self.optional_attributes

        create_output(simulation, obj)
        sync_output(simulation, obj, self.num_colliders)

        context.collection.objects.link(obj)

        self.report(
            {"INFO"},
            f"Added {obj.name} to output objects of {simulation.name}.",
        )
        return {"FINISHED"}

    def invoke(self, context, _):
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        self.layout.label(text=f"{self.output_type}")
        self.layout.prop(self, "object_name")
        draw_object_attributes(self.layout, self.output_type, self.optional_attributes)
        tutorial_msg(
            self.layout,
            context,
            """\
            You're about to add an output for the simulation.

            This creates a new object that is a receiver
            of the simulation results.
            As you play the animation in Blender,
            the receiver's data is loaded from the cache.

            A default visualization is provided as a
            geometry nodes modifier and it'll use the
            attributes that are selected by default.

            So, you can just press OK.""",
        )


class OBJECT_OT_Blended_MPM_Remove_Output_Object(bpy.types.Operator):
    bl_idname = "object.blended_mpm_remove_output_object"
    bl_label = "Remove Output Object"
    bl_description = """Deactivates the selected object as a simulation output.

Note that this does not delete the object."""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return (
            context.mode == "OBJECT" and get_selected_output_object(context) is not None
        )

    def execute(self, context):
        obj = get_selected_output_object(context)
        obj.blended_mpm_object.simulation_uuid = ""
        remove_drivers(obj)

        self.report(
            {"INFO"},
            f"Removed {obj.name} from output objects.",
        )
        return {"FINISHED"}


class OBJECT_UL_Blended_MPM_Output_Object_List(bpy.types.UIList):
    def filter_items(self, context, _data, _property):
        simulation = get_selected_simulation(context)
        if simulation is None:
            return [0] * len(context.scene.objects), []

        output_objects = get_output_objects(simulation)
        return [
            self.bitflag_filter_item if obj in output_objects else 0
            for obj in context.scene.objects
        ], []

    def draw_item(
        self,
        _context,
        layout,
        _data,
        obj,
        _icon,
        _active_data,
        _active_property,
    ):
        layout.label(text=obj.name)


class OBJECT_PT_Blended_MPM_Output(bpy.types.Panel):
    bl_label = "Output"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Blended MPM"
    bl_options = set()

    @classmethod
    def poll(cls, context):
        simulation = get_selected_simulation(context)
        return (
            context.mode == "OBJECT"
            and simulation is not None
            and context_exists(simulation)
            and available_frames(simulation) > 0
        )

    def draw(self, context):
        simulation = get_selected_simulation(context)
        self.layout.prop(simulation, "display_start_frame")
        if simulation.loaded_frame == -1:
            tut = self.layout.column()
            tut.alert = context.scene.blended_mpm_scene.tutorial_active
            tut.operator("object.blended_mpm_jump_to_start")
            return

        col = self.layout.column()
        col.enabled = False
        col.prop(simulation, "loaded_frame")

        row = self.layout.row()
        row.column().template_list(
            "OBJECT_UL_Blended_MPM_Output_Object_List",
            "",
            context.scene,
            "objects",
            context.scene.blended_mpm_scene,
            "selected_output_object",
        )
        list_controls = row.column(align=True)
        list_controls.operator(
            "object.blended_mpm_remove_output_object", text="", icon="REMOVE"
        )

        input_names = InputNames(simulation)
        num_colliders = len(input_names.collider_names)

        def create_operator(layout, output_type, icon, input_name):
            op = layout.operator(
                "object.blended_mpm_add_output_object", text=input_name, icon=icon
            )
            op.object_name = f"{output_type} - {input_name}"
            op.output_type = output_type
            op.input_name = input_name
            op.num_colliders = num_colliders

        if input_names.solid_names:
            box = self.layout.box()
            box.label(text="Add Solid Output")
            for name in input_names.solid_names:
                tut = box.column()
                tut.alert = context.scene.blended_mpm_scene.tutorial_active
                create_operator(tut, SOLID_PARTICLES, "POINTCLOUD_DATA", name)
        if input_names.fluid_names:
            box = self.layout.box()
            box.label(text="Add Fluid Output")
            for name in input_names.fluid_names:
                create_operator(box, FLUID_PARTICLES, "POINTCLOUD_DATA", name)
        if input_names.collider_names:
            box = self.layout.box()
            box.label(text="Add Collider Output")
            for name in input_names.collider_names:
                create_operator(box, COLLIDER_SAMPLES, "POINTCLOUD_DATA", name)
                create_operator(box, COLLIDER_MESH, "MESH_DATA", name)
        box = self.layout.box()
        box.label(text="Add Grid Output")
        create_operator(box, GRID_COLLIDER_DISTANCE, "MESH_GRID", "Distances")
        create_operator(box, GRID_MOMENTUM_FREE, "MESH_GRID", "Free Momentum")
        for name in input_names.collider_names:
            create_operator(box, GRID_MOMENTUM_CONFORMED, "MESH_GRID", name)
        box = self.layout.box()
        box.label(text="Get Original Input Meshes")
        for name in input_names.mesh_names:
            create_operator(box, INPUT_MESH, "MESH_DATA", name)


classes = [
    OBJECT_OT_Blended_MPM_Jump_To_Start,
    OBJECT_OT_Blended_MPM_Add_Output_Object,
    OBJECT_OT_Blended_MPM_Remove_Output_Object,
    OBJECT_UL_Blended_MPM_Output_Object_List,
    OBJECT_PT_Blended_MPM_Output,
]


def register_panel_output():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_output():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
