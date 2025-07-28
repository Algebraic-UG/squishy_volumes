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

from ..properties.util import (
    get_input_objects,
    get_selected_input_object,
    get_selected_simulation,
    get_simulation_specific_settings,
    has_simulation_specific_settings,
)
from ..properties.blended_mpm_object_settings import (
    OBJECT_ENUM_COLLIDER,
    OBJECT_ENUM_FLUID,
    OBJECT_ENUM_SOLID,
    Blended_MPM_Object_Settings,
    current_input_names_match_cached,
    get_input_solids,
)
from ..bridge import available_frames, context_exists, new_simulation
from ..setup import create_setup_json, is_scripted
from ..frame_change import (
    register_frame_handler,
    unregister_frame_handler,
)
from ..util import (
    copy_simple_property_group,
    force_ui_redraw,
    simulation_cache_exists,
    tutorial_msg,
)
from ..popup import with_popup

from ..properties.blended_mpm_object_settings import get_input_colliders


def draw_object_settings(layout, settings):
    layout.prop(settings, "object_enum")
    match settings.object_enum:
        case e if e == OBJECT_ENUM_SOLID:
            layout.prop(settings, "density")
            layout.prop(settings, "youngs_modulus")
            layout.prop(settings, "poissons_ratio")
            layout.prop(settings, "dilation")
            layout.prop(settings, "randomness")
            layout.prop(settings, "initial_linear_velocity")
            layout.prop(settings, "initial_angular_velocity")
        case e if e == OBJECT_ENUM_FLUID:
            layout.prop(settings, "density")
            layout.prop(settings, "exponent")
            layout.prop(settings, "bulk_modulus")
            layout.prop(settings, "dilation")
            layout.prop(settings, "randomness")
            layout.prop(settings, "initial_linear_velocity")
            layout.prop(settings, "initial_angular_velocity")
        case e if e == OBJECT_ENUM_COLLIDER:
            layout.prop(settings, "sticky_factor")
            layout.prop(settings, "friction_factor")


def selection_eligible_for_input(context):
    return (
        get_selected_simulation(context) is not None
        and context.active_object is not None
        and context.active_object.select_get()
        and context.active_object.type == "MESH"
        # This could be allowed?
        and not context.active_object.blended_mpm_object.simulation_uuid
        and not has_simulation_specific_settings(
            get_selected_simulation(context), context.active_object
        )
    )


class OBJECT_OT_Blended_MPM_Add_Input_Object(bpy.types.Operator):
    bl_idname = "object.blended_mpm_add_input_object"
    bl_label = "Add Input Object"
    bl_description = """Add the selected mesh object to the list of inputs.

If the input is not a collider, it will be sampled with particles
when the initial state is constructed.
To this end, it is important that the mesh is somewhat closed and oriented.

An active output object cannot be used as input.
Note that an eligible object must be selected."""
    bl_options = {"REGISTER", "UNDO"}

    settings: bpy.props.PointerProperty(type=Blended_MPM_Object_Settings)  # type: ignore

    @classmethod
    def poll(cls, context):
        return selection_eligible_for_input(context)

    def execute(self, context):
        settings = context.object.blended_mpm_object.simulation_specific_settings.add()
        copy_simple_property_group(self.settings, settings)

        simulation = get_selected_simulation(context)
        settings.simulation_uuid = simulation.uuid

        force_ui_redraw()

        self.report(
            {"INFO"},
            f"Added {context.object.name} to input objects of {simulation.name}.",
        )
        return {"FINISHED"}

    def invoke(self, context, _):
        return context.window_manager.invoke_props_dialog(self)

    def draw(self, context):
        simulation = get_selected_simulation(context)
        self.layout.label(text=context.object.name)
        draw_object_settings(self.layout, self.settings)

        # tutorial
        msg = "You're about to register the selected object as input."

        added_solid = bool(get_input_solids(simulation))
        added_collider = bool(get_input_colliders(simulation))
        if not added_solid or not added_collider:
            msg = (
                msg
                + f"""

                For the *Type* select *{"Collider" if added_solid else "Solid"}*.

                You can leave the settings default
                and hit OK!"""
            )
        tutorial_msg(self.layout, context, msg)


class OBJECT_OT_Blended_MPM_Remove_Input_Object(bpy.types.Operator):
    bl_idname = "object.blended_mpm_remove_input_object"
    bl_label = "Remove"
    bl_description = """Remove the selected object from the list of inputs.

Note that this does not delete the object."""
    bl_options = {"REGISTER", "UNDO"}

    @classmethod
    def poll(cls, context):
        return (
            context.mode == "OBJECT"
            and get_selected_simulation(context) is not None
            and get_selected_input_object(context) is not None
        )

    def execute(self, context):
        simulation = get_selected_simulation(context)
        obj = get_selected_input_object(context)
        simulation_specific_settings = (
            obj.blended_mpm_object.simulation_specific_settings
        )
        simulation_specific_settings.remove(
            [
                idx
                for idx, settings in enumerate(simulation_specific_settings)
                if settings.simulation_uuid == simulation.uuid
            ][0]
        )
        self.report(
            {"INFO"}, f"Removed {obj.name} from input objects of {simulation.name}."
        )
        return {"FINISHED"}


class OBJECT_OT_Blended_MPM_Write_Input_To_Cache(bpy.types.Operator):
    bl_idname = "object.blended_mpm_write_input_to_cache"
    bl_label = "Write to Cache"
    bl_description = """(Over)Write the cache with the new input.

This writes global settings as well as object specific settings
to the simulation cache.

Note that this also discards all computed frames in the cache."""
    bl_options = {"REGISTER"}

    def execute(self, context):
        simulation = get_selected_simulation(context)

        unregister_frame_handler()
        frame_current = context.scene.frame_current

        setup_json = with_popup(simulation, lambda: create_setup_json(simulation))

        context.scene.frame_set(frame_current)
        register_frame_handler()

        if not setup_json:
            return {"CANCELLED"}

        simulation.last_exception = ""
        simulation.loaded_frame = -1

        with_popup(simulation, lambda: new_simulation(simulation, setup_json))

        self.report({"INFO"}, f"Updating cache of {simulation.name}")
        return {"FINISHED"}

    def invoke(self, context, _):
        if context.scene.blended_mpm_scene.tutorial_active or simulation_cache_exists(
            get_selected_simulation(context)
        ):
            return context.window_manager.invoke_props_dialog(self)
        else:
            return self.execute(context)

    def draw(self, context):
        simulation = get_selected_simulation(context)
        if simulation_cache_exists(simulation):
            self.layout.label(text="WARNING: This is a destructive operation!")
            self.layout.label(
                text=f"The previous cache will be overwritten: {available_frames(simulation)} frames"
            )
        tutorial_msg(
            self.layout,
            context,
            """\
            You're about to write your input to cache and
            complete preparations to get simulating!

            From this step onwards, the simulation is going to
            remember this *current* input state.

            So, if you wish to change the simulation, any
            changes to settings, animation, geometry, etc.
            mandate to *Overwrite Cache* again.

            Please keep this in mind and hit OK.""",
        )


class OBJECT_UL_Blended_MPM_Input_Object_List(bpy.types.UIList):
    def filter_items(self, context, _data, _property):
        simulation = get_selected_simulation(context)
        if simulation is None:
            return [0] * len(context.scene.objects), []

        input_objects = get_input_objects(simulation)
        return [
            self.bitflag_filter_item if obj in input_objects else 0
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


class OBJECT_PT_Blended_MPM_Input(bpy.types.Panel):
    bl_label = "Input"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Blended MPM"  # The tab name
    bl_options = set()

    @classmethod
    def poll(cls, context):
        return context.mode == "OBJECT" and get_selected_simulation(context) is not None

    def draw(self, context):
        simulation = get_selected_simulation(context)

        (header, body) = self.layout.panel("settings", default_closed=True)
        header.label(text="Settings")
        if body is not None:
            row = body.row()
            to_cache = row.column()
            to_cache.prop(simulation.to_cache, "grid_node_size")
            to_cache.prop(simulation.to_cache, "particle_factor")
            particle_size = to_cache.column()
            particle_size.enabled = False
            particle_size.prop(simulation.to_cache, "particle_size")
            to_cache.prop(simulation.to_cache, "frames_per_second")
            to_cache.prop(simulation.to_cache, "gravity")

            if context_exists(simulation):
                from_cache = row.column()
                from_cache.enabled = False
                from_cache.prop(simulation.from_cache, "grid_node_size")
                from_cache.prop(simulation.from_cache, "particle_factor")
                from_cache.prop(simulation.from_cache, "particle_size")
                from_cache.prop(simulation.from_cache, "frames_per_second")
                from_cache.prop(simulation.from_cache, "gravity")

        obj = get_selected_input_object(context)
        if obj is not None:
            (header, body) = self.layout.panel(
                "input_object_settings", default_closed=True
            )
            header.label(text=f"Settings for {obj.name}")
            if body is not None:
                draw_object_settings(
                    body, get_simulation_specific_settings(simulation, obj)
                )

        row = self.layout.row()
        row.column().template_list(
            "OBJECT_UL_Blended_MPM_Input_Object_List",
            "",
            context.scene,
            "objects",
            context.scene.blended_mpm_scene,
            "selected_input_object",
        )
        list_controls = row.column(align=True)
        tut = list_controls.column()
        tut.alert = context.scene.blended_mpm_scene.tutorial_active and (
            not get_input_solids(simulation) or not get_input_colliders(simulation)
        )
        tut.operator("object.blended_mpm_add_input_object", text="", icon="ADD")
        list_controls.operator(
            "object.blended_mpm_remove_input_object", text="", icon="REMOVE"
        )

        if any([is_scripted(simulation, obj) for obj in get_input_objects(simulation)]):
            self.layout.prop(simulation, "capture_start_frame")
            self.layout.prop(simulation, "capture_frames")
            self.layout.separator()

        tut = self.layout.column()
        tut.alert = (
            context.scene.blended_mpm_scene.tutorial_active
            and get_input_solids(simulation)
            and get_input_colliders(simulation)
            and not current_input_names_match_cached(simulation)
        )
        tut.operator(
            "object.blended_mpm_write_input_to_cache",
            text=(
                "Overwrite Cache"
                if simulation_cache_exists(simulation)
                else "Initialize Cache"
            ),
            icon="FILE_CACHE",
        )


classes = [
    OBJECT_OT_Blended_MPM_Add_Input_Object,
    OBJECT_OT_Blended_MPM_Remove_Input_Object,
    OBJECT_OT_Blended_MPM_Write_Input_To_Cache,
    OBJECT_UL_Blended_MPM_Input_Object_List,
    OBJECT_PT_Blended_MPM_Input,
]


def register_panel_input():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_input():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
