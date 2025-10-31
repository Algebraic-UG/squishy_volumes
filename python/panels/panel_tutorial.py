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

import textwrap

from ..magic_consts import SOLID_PARTICLES
from ..bridge import InputNames, available_frames, computing, context_exists
from ..util import simulation_cache_exists
from ..properties.util import get_output_objects, get_selected_simulation
from ..properties.squishy_volumes_object_settings import (
    get_input_colliders,
    get_input_solids,
)

from .panel_input import selection_eligible_for_input
from .panel_overview import (
    SCENE_OT_Squishy_Volumes_Add_Simulation,
)


class OBJECT_OT_Squishy_Volumes_Start_Tutorial(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_start_tutorial"
    bl_label = "Start Tutorial"
    bl_description = """The tutorial helps you to execute a basic workflow.

When the tutorial is activated, you are guided
to create a simple simluation of a rubber block
that is dropped onto a plane."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        context.scene.squishy_volumes_scene.tutorial_active = True
        return {"FINISHED"}


class OBJECT_OT_Squishy_Volumes_Stop_Tutorial(bpy.types.Operator):
    bl_idname = "object.squishy_volumes_stop_tutorial"
    bl_label = "Stop Tutorial"
    bl_description = "Removes the hints and highlighting."
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        context.scene.squishy_volumes_scene.tutorial_active = False
        return {"FINISHED"}


def current_instructions(layout, context):
    def display_msg(msg):
        for line in textwrap.dedent(msg).splitlines():
            layout.label(text=line)

    if not context.scene.squishy_volumes_scene.simulations:
        display_msg(
            f"""\
            🌱 Welcome to the tutorial! 🌱

            Follow the highlighted buttons
            and text instructions.

            Please start by adding a new simulation.
            Press {SCENE_OT_Squishy_Volumes_Add_Simulation.bl_label}!"""
        )
        return

    simulation = get_selected_simulation(context)

    if simulation_cache_exists(simulation) and not context_exists(simulation):
        display_msg(
            f"""\
            Small issue 🙁
            The cache directory
            '{simulation.cache_directory}'
            was already in use.

            This is normally ok, but the tutorial
            expects a fresh start!

            Please either remove the contents
            (with your favorite filebrowser)
            or choose a different *Cache*."""
        )
        return

    if not get_input_solids(simulation):
        display_msg(
            """\
            Now you can add some inputs!
            Start by adding a *Solid*.

            """
        )
        if not selection_eligible_for_input(context):
            display_msg(
                """\
                Select any mesh that has some volume
                (care for face orientation)
                and isn't already input to this
                simulation.

                The simplest way is to select the
                default cube if you still have it."""
            )
        else:
            display_msg(
                """\
                Add the selected object as input:
                Press the ＋ button!"""
            )

        return

    if not get_input_colliders(simulation):
        display_msg(
            """\
            Now also add a *Collider*.

            """
        )
        if not selection_eligible_for_input(context):
            display_msg(
                """\
                Select any mesh that isn't already
                an input to the simulation.

                For example, create a simple plane
                under the solid input."""
            )
        else:
            display_msg(
                """\
                Add the selected object as input:
                Press the ＋ button!"""
            )

        return

    if not context_exists(simulation):
        display_msg(
            """\
            Great, the input is defined!

            Now we'll give the simulation engine
            the input list.

            Press Initialize Cache!"""
        )
        return

    if not computing(simulation) and available_frames(simulation) == 0:
        display_msg(
            """\
            We need at least one frame to continue.

            Please press either
            Overwrite Cache
            or
            Create Simulation State."""
        )
        return

    if simulation.loaded_frame == -1:
        display_msg(
            """\
            At least one frame is ready!

            Switch to a frame that is already
            computed.

            Go to the Output panel and press
            Jump to First Frame.
            """
        )
        return

    input_names = InputNames(simulation, simulation.loaded_frame)
    if not [
        obj
        for obj in get_output_objects(simulation)
        if obj.squishy_volumes_object.output_type == SOLID_PARTICLES
    ]:
        display_msg(
            f"""\
            At least one frame is ready!

            Let's see the results by
            defining some *Output*.

            Go to the Output panel.
            You might need to scroll
            or minimize panels.

            Under *Solids* find and
            press {next(iter(input_names.solid_names))}!
            """
        )
        return

    if not context.screen.is_animation_playing:
        display_msg(
            """\
            You're almost there!

            The only thing left to do is to
            play the animation!

            Either press SPACE
            or the play button."""
        )
        return

    display_msg(
        """\
        🎉 You have completed the tutorial! 🎉

        Summary:
        1. Create simulation
        2. Add input
        3. Write cache & simulate
        4. Add output

        If you want to continue learning
        about the features visit
        TODO: links"""
    )


class OBJECT_PT_Squishy_Volumes_Tutorial(bpy.types.Panel):
    bl_label = "Tutorial"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Squishy Volumes"

    @classmethod
    def poll(cls, context):
        return (
            not context.scene.squishy_volumes_scene.simulations
            or context.scene.squishy_volumes_scene.tutorial_active
        )

    def draw(self, context):
        if context.scene.squishy_volumes_scene.tutorial_active:
            current_instructions(self.layout.box(), context)
            self.layout.operator("object.squishy_volumes_stop_tutorial")
        else:
            self.layout.operator("object.squishy_volumes_start_tutorial")


classes = [
    OBJECT_OT_Squishy_Volumes_Start_Tutorial,
    OBJECT_OT_Squishy_Volumes_Stop_Tutorial,
    OBJECT_PT_Squishy_Volumes_Tutorial,
]


def register_panel_tutorial():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_tutorial():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
