import bpy

import textwrap

from ..util import simulation_cache_exists
from ..bridge import InputNames

from ..properties.blended_mpm_object_settings import (
    get_input_colliders,
    get_input_fluids,
    get_input_solids,
)
from ..properties.util import get_input_objects

from .panel_input import selection_eligible_for_input
from .panel_overview import (
    OBJECT_OT_Blended_MPM_Add_Simulation,
    OBJECT_OT_Blended_MPM_Remove_Simulation,
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
            Please start by adding a new simulation.
            Press {OBJECT_OT_Blended_MPM_Add_Simulation.bl_label}!"""
        )
        return

    if len(context.scene.blended_mpm_scene.simulations) > 1:
        layout.alert = True
        display_msg(
            f"""\
            You have added several simulations.

            This is supported in general!
            But the tutorial does not account for it.

            Please remove all but one:
            Expand the simulations and press
            {OBJECT_OT_Blended_MPM_Remove_Simulation.bl_label}!"""
        )
        return

    simulation = context.scene.blended_mpm_scene.simulations[0]

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
                Press the *plus* button!"""
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
                Press the *plus* button!"""
            )

        return

    solid_names = {obj.name for obj in get_input_solids(simulation)}
    fluid_names = {obj.name for obj in get_input_fluids(simulation)}
    collider_names = {obj.name for obj in get_input_colliders(simulation)}
    input_names = InputNames(simulation)

    names_differ = (
        input_names.solid_names != solid_names
        or input_names.fluid_names != fluid_names
        or input_names.collider_names != collider_names
    )

    if names_differ:
        display_msg(
            f"""\
            Great, the input is defined!

            Now we'll give the simulation engine
            the list.

            From this step onwards, the simulation
            is going to remember this input state,
            so if we want to change something we
            need to *Overwrite Cache*!

            Press {"Overwrite Cache" if simulation_cache_exists(simulation) else "Initialize Cache"}!"""
        )
        return

    display_msg("You have completed the tutorial!")


class OBJECT_PT_Blended_MPM_Tutorial(bpy.types.Panel):
    bl_label = "Tutorial"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Blended MPM"  # The tab name

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
