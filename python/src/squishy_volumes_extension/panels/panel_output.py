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

from typing import Any


from ..frame_change import sync_simulation
from ..util import copy_simple_property_group

from ..magic_consts import (
    GRID,
    PARTICLES,
    OUTPUT_TYPES,
)


from ..squishy_volumes_properties import (
    get_selected_simulation_uuid,
    INPUT_TYPE_PARTICLES,
    INPUT_TYPE_COLLIDER,
    get_output_objects,
    TYPE_OUTPUT,
    TYPE_NONE,
    get_output_objects_with_uuid,
    get_selected_simulation_object,
    get_selected_output_object,
    Squishy_Volumes_Properties_Output,
    add_fields_from,
    get_simulation_object_with_uuid,
)
from ..output import (
    create_default_visualization,
    sync_output,
)
from ..bridge import SimulationHandle


class Squishy_Volumes_New_Output_Object(bpy.types.PropertyGroup):
    input_name: bpy.props.StringProperty(
        name="Input Name",
        description="The original name of the given input object.",
    )  # type: ignore
    output_name: bpy.props.StringProperty(
        name="Output Name",
        description="The new name for the new output object.",
    )  # type: ignore
    select: bpy.props.BoolProperty(
        name="Add", description="Create an output for this input.", default=True
    )  # type: ignore


class SCENE_UL_Squishy_Volumes_New_Output_Object_List(bpy.types.UIList):
    def draw_item(
        self,
        context,
        layout,
        data,
        item,
        icon,
        active_data,
        active_property,
        index,
        flt_flag,
    ):
        assert isinstance(item, Squishy_Volumes_New_Output_Object)
        row = layout.row()
        row.prop(item, "select", text=item.input_name)
        row.prop(item, "output_name")


def update_select_action(self, context):
    if self.select_action == "All":
        for output in self.particle_outputs:
            output.select = True
    if self.select_action == "None":
        for output in self.particle_outputs:
            output.select = False


@add_fields_from(Squishy_Volumes_Properties_Output)
class SCENE_OT_Squishy_Volumes_Add_Output_Object(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_output_object"
    bl_label = "Add Output Object"
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore

    output_name: bpy.props.StringProperty()  # type: ignore
    add_default_visualization: bpy.props.BoolProperty()  # type:ignore

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)

        if self.output_type != GRID:  # ty:ignore[unresolved-attribute]
            if self.input_name not in bpy.data.objects:  # ty:ignore[unresolved-attribute]
                self.report(
                    {"WARNING"},
                    f"Couldn't find input object '{self.input_name}', skinning might not work.",  # ty:ignore[unresolved-attribute]
                )

        output_obj = bpy.data.objects.new(
            self.output_name, bpy.data.meshes.new(self.output_name)
        )
        context.collection.objects.link(output_obj)

        output_props = output_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        output_props.type = TYPE_OUTPUT
        output_props.uuid = self.uuid

        copy_simple_property_group(self, output_props)

        if self.add_default_visualization:
            create_default_visualization(sim_obj, output_obj)

        self.report(
            {"INFO"},
            f"Added {output_obj.name} to output objects of {sim_obj.name}.",
        )

        return {"FINISHED"}


@add_fields_from(Squishy_Volumes_Properties_Output)
class SCENE_OT_Squishy_Volumes_Add_Output_Objects(bpy.types.Operator):
    bl_idname = "scene.squishy_volumes_add_output_objects"
    bl_label = "Add Output Objects"
    bl_description = """Create a new active output object from the simulation cache.

There are several output object types.
Most outputs are point-based and the object will be populated
with certain attributes and geometry node modifier.

As long as the object is an active output
and the current frame is availbe in the cache,
the positions and attributes are synchronized
each frame."""
    bl_options = {"REGISTER", "UNDO"}

    uuid: bpy.props.StringProperty()  # type: ignore

    particle_outputs: bpy.props.CollectionProperty(
        type=Squishy_Volumes_New_Output_Object
    )  # type: ignore
    collider_outputs: bpy.props.CollectionProperty(
        type=Squishy_Volumes_New_Output_Object
    )  # type: ignore
    selected_output: bpy.props.IntProperty()  # type: ignore

    select_action: bpy.props.EnumProperty(
        items=[
            ("All",) * 3,
            ("Custom",) * 3,
            ("None",) * 3,
        ],  # ty:ignore[invalid-argument-type]
        update=update_select_action,
        default="All",
    )  # type: ignore

    add_default_visualization: bpy.props.BoolProperty(
        name="Add Default Visualization", default=True
    )  # type:ignore

    def execute(self, context):
        sim_obj = get_simulation_object_with_uuid(self.uuid)

        if self.output_type == GRID:  # ty:ignore[unresolved-attribute]
            bpy.ops.scene.squishy_volumes_add_output_object(  # ty:ignore[unresolved-attribute]
                "INVOKE_DEFAULT",
                uuid=self.uuid,
                input_name="",
                output_name="Grid - Output",
                add_default_visualization=self.add_default_visualization,
                output_type=self.output_type,  # ty:ignore[unresolved-attribute]
                grid_collider_bits=self.grid_collider_bits,  # ty:ignore[unresolved-attribute]
                grid_masses=self.grid_masses,  # ty:ignore[unresolved-attribute]
                grid_velocities=self.grid_velocities,  # ty:ignore[unresolved-attribute]
                particle_flags=self.particle_flags,  # ty:ignore[unresolved-attribute]
                particle_masses=self.particle_masses,  # ty:ignore[unresolved-attribute]
                particle_initial_volumes=self.particle_initial_volumes,  # ty:ignore[unresolved-attribute]
                particle_initial_positions=self.particle_initial_positions,  # ty:ignore[unresolved-attribute]
                particle_velocities=self.particle_velocities,  # ty:ignore[unresolved-attribute]
                particle_sizes=self.particle_sizes,  # ty:ignore[unresolved-attribute]
                particle_transformations=self.particle_transformations,  # ty:ignore[unresolved-attribute]
                particle_energies=self.particle_energies,  # ty:ignore[unresolved-attribute]
                particle_collider_bits=self.particle_collider_bits,  # ty:ignore[unresolved-attribute]
            )

        if self.output_type == PARTICLES:  # ty:ignore[unresolved-attribute]
            for output in self.particle_outputs:
                if not output.select:
                    continue

                bpy.ops.scene.squishy_volumes_add_output_object(  # ty:ignore[unresolved-attribute]
                    "INVOKE_DEFAULT",
                    uuid=self.uuid,
                    input_name=output.input_name,
                    output_name=output.output_name,
                    add_default_visualization=self.add_default_visualization,
                    output_type=self.output_type,  # ty:ignore[unresolved-attribute]
                    grid_collider_bits=self.grid_collider_bits,  # ty:ignore[unresolved-attribute]
                    grid_masses=self.grid_masses,  # ty:ignore[unresolved-attribute]
                    grid_velocities=self.grid_velocities,  # ty:ignore[unresolved-attribute]
                    particle_flags=self.particle_flags,  # ty:ignore[unresolved-attribute]
                    particle_masses=self.particle_masses,  # ty:ignore[unresolved-attribute]
                    particle_initial_volumes=self.particle_initial_volumes,  # ty:ignore[unresolved-attribute]
                    particle_initial_positions=self.particle_initial_positions,  # ty:ignore[unresolved-attribute]
                    particle_velocities=self.particle_velocities,  # ty:ignore[unresolved-attribute]
                    particle_sizes=self.particle_sizes,  # ty:ignore[unresolved-attribute]
                    particle_transformations=self.particle_transformations,  # ty:ignore[unresolved-attribute]
                    particle_energies=self.particle_energies,  # ty:ignore[unresolved-attribute]
                    particle_collider_bits=self.particle_collider_bits,  # ty:ignore[unresolved-attribute]
                )

        sim_handle = SimulationHandle.get(uuid=self.uuid)
        if sim_handle is not None:
            sync_simulation(
                sim_props=sim_obj.squishy_volumes,  # ty:ignore[unresolved-attribute]
                sim_handle=sim_handle,
                frame=context.scene.frame_current,
            )

        return {"FINISHED"}

    def invoke(self, context, event):
        self.particle_outputs.clear()
        self.collider_outputs.clear()
        sim_handle = SimulationHandle.get(uuid=self.uuid)
        if sim_handle is None:
            return {"CANCELLED"}
        input_header = sim_handle.input_header()
        for name, obj in input_header["objects"].items():
            if INPUT_TYPE_PARTICLES in obj:
                output = self.particle_outputs.add()
            elif INPUT_TYPE_COLLIDER in obj:
                output = self.collider_outputs.add()
            else:
                continue
            output.input_name = name
            output.output_name = name + " - Output"

        return context.window_manager.invoke_props_dialog(self, width=600)

    def draw_selection_list(self):
        assert isinstance(self.layout, bpy.types.UILayout)
        propname = {
            PARTICLES: "particle_outputs",
        }.get(self.output_type)  # ty:ignore[unresolved-attribute]
        if propname is None:
            return
        self.layout.prop(self, "select_action", expand=True)
        self.layout.template_list(
            listtype_name="SCENE_UL_Squishy_Volumes_New_Output_Object_List",
            list_id="",
            dataptr=self,
            propname=propname,
            active_dataptr=self,
            active_propname="selected_output",
        )

    def draw_object_attributes(self):
        assert isinstance(self.layout, bpy.types.UILayout)
        output_type = self.output_type  # ty:ignore[unresolved-attribute]

        box = self.layout.box()
        box.label(text="These attributes will be loaded each frame.")
        box.label(
            text="The default selection is needed by the default visualization, but you might need less!"
        )
        box.label(text="Please mouse-over for the exact identifier.")
        grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
        grid.label(text="Attribute")
        grid.label(text="Type")
        if output_type == GRID:
            grid.prop(self, "grid_collider_bits")
            grid.label(text="FLOAT")
            grid.prop(self, "grid_masses")
            grid.label(text="FLOAT")
            grid.prop(self, "grid_velocities")
            grid.label(text="FLOAT_VECTOR")
        if output_type == PARTICLES:
            grid.prop(self, "particle_flags")
            grid.label(text="INT")
            grid.prop(self, "particle_masses")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_initial_volumes")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_initial_positions")
            grid.label(text="FLOAT_VECTOR")
            grid.prop(self, "particle_velocities")
            grid.label(text="FLOAT_VECTOR")
            grid.prop(self, "particle_sizes")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_transformations")
            grid.label(text="FLOAT4X4")
            grid.prop(self, "particle_energies")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_collider_bits")
            grid.label(text="FLOAT")

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        self.layout.prop(self, "output_type")

        self.draw_selection_list()
        self.draw_object_attributes()

        self.layout.prop(self, "add_default_visualization")

        if (
            self.select_action == "None"
            and any(o.select for o in self.particle_outputs)
        ) or (
            self.select_action == "All"
            and any(not o.select for o in self.particle_outputs)
        ):
            self.select_action = "Custom"


class OBJECT_OT_Squishy_Volumes_Remove_Output_Object(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_remove_output_object"
    bl_label = "Remove Output Object"
    bl_description = """Deactivates the selected object as a simulation output.

Note that this does not delete the object."""
    bl_options = {"REGISTER", "UNDO"}

    name: bpy.props.StringProperty()  # type: ignore

    def execute(self, context):
        output_obj = bpy.data.objects[self.name]
        output_obj.squishy_volumes.uuid = "unassigned"
        output_obj.squishy_volumes.type = TYPE_NONE
        self.report(
            {"INFO"},
            f"Removed {output_obj.name} from output objects.",
        )
        return {"FINISHED"}


class SCENE_UL_Squishy_Volumes_Output_Object_List(bpy.types.UIList):
    def filter_items(self, context, data, property):
        uuid = get_selected_simulation_uuid(context.scene)
        if uuid is None:
            return [0] * len(bpy.data.objects), []

        output_objects = get_output_objects_with_uuid(uuid)
        return [
            self.bitflag_filter_item if obj in output_objects else 0
            for obj in bpy.data.objects
        ], []

    def draw_item(
        self,
        context,
        layout,
        data,
        item,
        icon,
        active_data,
        active_property,
        index,
        flt_flag,
    ):
        assert isinstance(item, bpy.types.Object)
        layout.label(text=item.name)


class SCENE_PT_Squishy_Volumes_Output(bpy.types.Panel):
    bl_label = "Output"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"
    bl_options = set()

    @classmethod
    def poll(cls, context):
        if context.mode != "OBJECT":
            return False

        uuid = get_selected_simulation_uuid(context.scene)
        return uuid is not None and SimulationHandle.exists(uuid=uuid)

    def draw(self, context):
        assert isinstance(self.layout, bpy.types.UILayout)
        sim_obj = get_selected_simulation_object(context.scene)
        sim_props = sim_obj.squishy_volumes  # ty:ignore[unresolved-attribute]
        self.layout.prop(sim_props, "display_start_frame")

        col = self.layout.column()
        if sim_props.has_loaded_frame:
            col.enabled = False
            col.prop(sim_props, "loaded_frame")
        else:
            col.label(text="No frame loaded")

        row = self.layout.row()
        row.column().template_list(
            "SCENE_UL_Squishy_Volumes_Output_Object_List",
            "",
            bpy.data,
            "objects",
            context.scene.squishy_volumes,
            "selected_output_object",
        )
        list_controls = row.column(align=True)
        add_op = list_controls.operator(
            SCENE_OT_Squishy_Volumes_Add_Output_Objects.bl_idname,
            text="",
            icon="ADD",
        ).uuid = sim_props.uuid

        remove = list_controls.column()
        remove_obj = get_selected_output_object(context.scene)
        if remove_obj is None:
            remove.enabled = False
            remove.operator(
                OBJECT_OT_Squishy_Volumes_Remove_Output_Object.bl_idname,
                text="",
                icon="REMOVE",
            )
        else:
            remove.operator(
                OBJECT_OT_Squishy_Volumes_Remove_Output_Object.bl_idname,
                text="",
                icon="REMOVE",
            ).name = remove_obj.name


classes = [
    Squishy_Volumes_New_Output_Object,
    SCENE_UL_Squishy_Volumes_New_Output_Object_List,
    SCENE_OT_Squishy_Volumes_Add_Output_Object,
    SCENE_OT_Squishy_Volumes_Add_Output_Objects,
    OBJECT_OT_Squishy_Volumes_Remove_Output_Object,
    SCENE_UL_Squishy_Volumes_Output_Object_List,
    SCENE_PT_Squishy_Volumes_Output,
]


def register_panel_output():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_output():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
