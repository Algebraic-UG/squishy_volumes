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
from ..util import copy_simple_property_group, frame_to_load

from ..nodes.drivers import remove_drivers
from ..magic_consts import (
    COLLIDER_SAMPLES,
    GRID_COLLIDER,
    GRID_MOMENTUM_CONFORMED,
    GRID_MOMENTUM_FREE,
    INPUT_MESH,
    PARTICLES,
    OUTPUT_TYPES,
)


from ..properties.squishy_volumes_object_input_settings import (
    INPUT_TYPE_PARTICLES,
    INPUT_TYPE_COLLIDER,
)
from ..properties.squishy_volumes_object import (
    get_output_objects,
    IO_OUTPUT,
)
from ..properties.squishy_volumes_scene import (
    get_selected_simulation,
    get_simulation_by_uuid,
    get_selected_output_object,
)
from ..properties.squishy_volumes_object_output_settings import (
    Squishy_Volumes_Object_Output_Settings,
)
from ..properties.squishy_volumes_simulation import Squishy_Volumes_Simulation
from ..properties.util import (
    add_fields_from,
)
from ..output import (
    create_default_visualization,
    sync_output,
)
from ..bridge import Simulation


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
        context: bpy.types.Context,
        layout: bpy.types.UILayout,
        data: Any | None,
        item: Any | None,
        icon: int | None,
        active_data: Any,
        active_property: str | None,
        index: int | None,
        flt_flag: int | None,
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


@add_fields_from(Squishy_Volumes_Object_Output_Settings)
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
        simulation = get_simulation_by_uuid(context.scene, self.uuid)
        assert isinstance(simulation, Squishy_Volumes_Simulation)

        def create_output_obj(*, output_name: str, input_name: str | None):
            obj = bpy.data.objects.new(output_name, bpy.data.meshes.new(output_name))
            context.collection.objects.link(obj)

            obj.squishy_volumes_object.io = IO_OUTPUT  # ty:ignore[unresolved-attribute]
            obj.squishy_volumes_object.simulation_uuid = self.uuid  # ty:ignore[unresolved-attribute]

            output_settings = obj.squishy_volumes_object.output_settings  # ty:ignore[unresolved-attribute]
            copy_simple_property_group(self, output_settings)

            if input_name is not None:
                output_settings.input_name = input_name

            if self.add_default_visualization:
                create_default_visualization(obj, self.uuid)

            self.report(
                {"INFO"},
                f"Added {obj.name} to output objects of {simulation.name}.",
            )

        if self.output_type == GRID_COLLIDER:  # ty:ignore[unresolved-attribute]
            create_output_obj(
                output_name="Collider Distances - Output", input_name=None
            )

        if self.output_type == GRID_MOMENTUM_FREE:  # ty:ignore[unresolved-attribute]
            create_output_obj(output_name="Grid Momentum - Output", input_name=None)

        if self.output_type == PARTICLES:  # ty:ignore[unresolved-attribute]
            for output in self.particle_outputs:
                if not output.select:
                    continue
                create_output_obj(
                    output_name=output.output_name,
                    input_name=output.input_name,
                )

        if self.output_type == GRID_MOMENTUM_CONFORMED:  # ty:ignore[unresolved-attribute]
            for output in self.collider_outputs:
                if not output.select:
                    continue
                create_output_obj(
                    output_name=output.output_name,
                    input_name=output.input_name,
                )

        sim = Simulation.get(uuid=self.uuid)
        if sim is not None:
            sync_simulation(
                sim=sim,
                simulation=simulation,
                frame=context.scene.frame_current,
            )

        return {"FINISHED"}

    def invoke(self, context, event):
        self.particle_outputs.clear()
        self.collider_outputs.clear()
        sim = Simulation.get(uuid=self.uuid)
        if sim is None:
            return {"CANCELLED"}
        input_header = sim.input_header()
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
        propname = {
            PARTICLES: "particle_outputs",
            GRID_MOMENTUM_CONFORMED: "collider_outputs",
        }.get(self.output_type)  # ty:ignore[unresolved-attribute]
        if propname is None:
            return
        self.layout.prop(self, "select_action", expand=True)  # ty:ignore[possibly-missing-attribute]
        self.layout.template_list(  # ty:ignore[possibly-missing-attribute]
            listtype_name="SCENE_UL_Squishy_Volumes_New_Output_Object_List",
            list_id="",
            dataptr=self,
            propname=propname,
            active_dataptr=self,
            active_propname="selected_output",
        )

    def draw_object_attributes(self):
        output_type = self.output_type  # ty:ignore[unresolved-attribute]
        if output_type == INPUT_MESH:
            return

        box = self.layout.box()  # ty:ignore[possibly-missing-attribute]
        box.label(text="These attributes will be loaded each frame.")
        box.label(
            text="The default selection is needed by the default visualization, but you might need less!"
        )
        box.label(text="Please mouse-over for the exact identifier.")
        grid = box.grid_flow(row_major=True, columns=2, even_columns=False)
        grid.label(text="Attribute")
        grid.label(text="Type")
        if output_type == GRID_COLLIDER:
            grid.prop(self, "grid_collider_distances")
            grid.label(text="FLOAT")
            grid.prop(self, "grid_collider_normals")
            grid.label(text="FLOAT_VECTOR")
            grid.prop(self, "grid_collider_velocities")
            grid.label(text="FLOAT_VECTOR")
        if output_type in [GRID_MOMENTUM_FREE, GRID_MOMENTUM_CONFORMED]:
            grid.prop(self, "grid_momentum_masses")
            grid.label(text="FLOAT")
            grid.prop(self, "grid_momentum_velocities")
            grid.label(text="FLOAT_VECTOR")
        if output_type == PARTICLES:
            grid.prop(self, "particle_states")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_masses")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_initial_volumes")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_initial_positions")
            grid.label(text="FLOAT_VECTOR")
            grid.prop(self, "particle_velocities")
            grid.label(text="FLOAT_VECTOR")
            grid.prop(self, "particle_transformations")
            grid.label(text="FLOAT4X4")
            grid.prop(self, "particle_energies")
            grid.label(text="FLOAT")
            grid.prop(self, "particle_collider_insides")
            grid.label(text="FLOAT")
        if output_type == COLLIDER_SAMPLES:
            grid.prop(self, "collider_normals")
            grid.label(text="FLOAT_VECTOR")
            grid.prop(self, "collider_velocities")
            grid.label(text="FLOAT_VECTOR")

    def draw(self, context):
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

    @classmethod
    def poll(cls, context):
        return (
            context.mode == "OBJECT"
            and get_selected_output_object(context.scene) is not None
        )

    def execute(self, context):
        obj = get_selected_output_object(context.scene)
        obj.squishy_volumes_object.simulation_uuid = ""
        remove_drivers(obj)

        self.report(
            {"INFO"},
            f"Removed {obj.name} from output objects.",
        )
        return {"FINISHED"}


class SCENE_UL_Squishy_Volumes_Output_Object_List(bpy.types.UIList):
    def filter_items(self, context, data, property):
        simulation = get_selected_simulation(context.scene)
        if simulation is None:
            return [0] * len(bpy.data.objects), []

        output_objects = get_output_objects(simulation)
        return [
            self.bitflag_filter_item if obj in output_objects else 0
            for obj in bpy.data.objects
        ], []

    def draw_item(
        self,
        context: bpy.types.Context,
        layout: bpy.types.UILayout,
        data: Any | None,
        item: Any | None,
        icon: int | None,
        active_data: Any,
        active_property: str | None,
        index: int | None,
        flt_flag: int | None,
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

        simulation = get_selected_simulation(context.scene)
        return simulation is not None and Simulation.exists(uuid=simulation.uuid)

    def draw(self, context):
        simulation = get_selected_simulation(context.scene)
        self.layout.prop(simulation, "display_start_frame")  # ty:ignore[possibly-missing-attribute]

        col = self.layout.column()  # ty:ignore[possibly-missing-attribute]
        if simulation.has_loaded_frame:
            col.enabled = False
            col.prop(simulation, "loaded_frame")
        else:
            col.label(text="No frame loaded")

        row = self.layout.row()  # ty:ignore[possibly-missing-attribute]
        row.column().template_list(
            "SCENE_UL_Squishy_Volumes_Output_Object_List",
            "",
            bpy.data,
            "objects",
            context.scene.squishy_volumes_scene,
            "selected_output_object",
        )
        list_controls = row.column(align=True)
        add_op = list_controls.operator(
            SCENE_OT_Squishy_Volumes_Add_Output_Objects.bl_idname,
            text="",
            icon="ADD",
        )
        add_op.uuid = simulation.uuid
        list_controls.operator(
            OBJECT_OT_Squishy_Volumes_Remove_Output_Object.bl_idname,
            text="",
            icon="REMOVE",
        )


classes = [
    Squishy_Volumes_New_Output_Object,
    SCENE_UL_Squishy_Volumes_New_Output_Object_List,
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
