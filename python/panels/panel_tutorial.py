import bpy

import textwrap

from ..magic_consts import SOLID_PARTICLES
from ..bridge import InputNames, available_frames, computing, context_exists
from ..util import simulation_cache_exists
from ..properties.util import get_output_objects
from ..properties.blended_mpm_object_settings import (
    get_input_colliders,
    get_input_solids,
)

from .panel_input import selection_eligible_for_input
from .panel_overview import (
    OBJECT_OT_Blended_MPM_Add_Simulation,
)


class OBJECT_OT_Blended_MPM_Start_Tutorial(bpy.types.Operator):
    bl_idname = "object.blended_mpm_start_tutorial"
    bl_label = "Start Tutorial"
    bl_description = """The tutorial helps you to execute a basic workflow.

When the tutorial is activated, you are guided
to create a simple simluation of a rubber block
that is dropped onto a plane."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        context.scene.blended_mpm_scene.tutorial_active = True
        return {"FINISHED"}


class OBJECT_OT_Blended_MPM_Stop_Tutorial(bpy.types.Operator):
    bl_idname = "object.blended_mpm_stop_tutorial"
    bl_label = "Stop Tutorial"
    bl_description = "Removes the hints and highlighting."
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        context.scene.blended_mpm_scene.tutorial_active = False
        return {"FINISHED"}


def current_instructions(layout, context):
    def display_msg(msg):
        for line in textwrap.dedent(msg).splitlines():
            layout.label(text=line)

    if not context.scene.blended_mpm_scene.simulations:
        display_msg(
            f"""\
            🌱 Welcome to the tutorial! 🌱

            Follow the highlighted buttons
            and text instructions.

            Please start by adding a new simulation.
            Press {OBJECT_OT_Blended_MPM_Add_Simulation.bl_label}!"""
        )
        return

    simulation = context.scene.blended_mpm_scene.simulations[0]

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
        if obj.blended_mpm_object.output_type == SOLID_PARTICLES
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


class OBJECT_PT_Blended_MPM_Tutorial(bpy.types.Panel):
    bl_label = "Tutorial"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Blended MPM"

    @classmethod
    def poll(cls, context):
        return (
            not context.scene.blended_mpm_scene.simulations
            or context.scene.blended_mpm_scene.tutorial_active
        )

    def draw(self, context):
        if context.scene.blended_mpm_scene.tutorial_active:
            current_instructions(self.layout.box(), context)
            self.layout.operator("object.blended_mpm_stop_tutorial")
        else:
            self.layout.operator("object.blended_mpm_start_tutorial")


classes = [
    OBJECT_OT_Blended_MPM_Start_Tutorial,
    OBJECT_OT_Blended_MPM_Stop_Tutorial,
    OBJECT_PT_Blended_MPM_Tutorial,
]


def register_panel_tutorial():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_tutorial():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
