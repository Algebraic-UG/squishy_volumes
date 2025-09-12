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

from pathlib import Path
import re
import tempfile
import bpy

from ..bridge import drop_context
from ..progress_update import cleanup_markers

from .squishy_volumes_simulation_settings import Squishy_Volumes_Simulation_Settings


def duplicate_simulation_name(simulation):
    simulations = bpy.context.scene.squishy_volumes_scene.simulations
    return any(
        [
            simulation.name == other.name
            for other in simulations
            if other.uuid != simulation.uuid
        ]
    )


def duplicate_simulation_cache_directory(simulation):
    simulations = bpy.context.scene.squishy_volumes_scene.simulations
    return any(
        [
            simulation.cache_directory == other.cache_directory
            for other in simulations
            if other.uuid != simulation.uuid
        ]
    )


def make_unique(new, existing):
    new = re.sub(r"\.\d\d\d$", "", new)
    for i in range(0, 999):
        trial = f"{new}.{i:03d}"
        if not trial in existing:
            return trial
    raise RuntimeError("Failed to make unique")


def update_name(self, context):
    if duplicate_simulation_name(self):
        self.name = make_unique(
            self.name, [s.name for s in context.scene.squishy_volumes_scene.simulations]
        )
        return  # we'll re-enter anyway

    cleanup_markers(self)


def update_cache_directory(self, context):
    if self.cache_directory != str(Path(self.cache_directory)):
        self.cache_directory = str(Path(self.cache_directory))
        return  # we'll re-enter anyway

    if duplicate_simulation_cache_directory(self):
        self.cache_directory = make_unique(
            self.cache_directory,
            [
                s.cache_directory
                for s in context.scene.squishy_volumes_scene.simulations
            ],
        )
        return  # we'll re-enter anyway

    cleanup_markers(self)
    drop_context(self)


class Squishy_Volumes_Simulation(bpy.types.PropertyGroup):
    name: bpy.props.StringProperty(
        name="Name",
        description="It is just the name wihtout any semantic implications.",
        default="My Simulation",
        options=set(),
        update=update_name,
    )  # type: ignore

    # ----------------------------------------------------------------
    # Updated in regular intervals
    # ----------------------------------------------------------------
    progress_json_string: bpy.props.StringProperty()  # type: ignore
    last_exception: bpy.props.StringProperty()  # type: ignore

    # ----------------------------------------------------------------
    # Ties to native context and disc
    # ----------------------------------------------------------------
    uuid: bpy.props.StringProperty(
        name="UUID",
        description="Readonly identifier that is used to reference this simulation.",
        default="unassigned",
        options=set(),
    )  # type: ignore
    cache_directory: bpy.props.StringProperty(
        name="Cache",
        description="""Directory that holds the relevant simulation data.
This includes settings, meshes, animations and simulated frames.
If there exists a cache at the location it can be loaded.

The directory will contain "setup.json", "frame_xxxxx.bin", and "lock".
The latter being a temporary file indicating ownership.""",
        default=str(Path(tempfile.gettempdir()) / "squishy_volumes_cache"),
        options=set(),
        update=update_cache_directory,
        subtype="DIR_PATH",
    )  # type: ignore
    max_giga_bytes_on_disk: bpy.props.FloatProperty(
        name="Max Diskspace (Gigabytes)",
        description="""Simulations can use a lot of disk space!

Once it is exceeded, the computation will stop.
Note that the limit can be violanted by a certain amount
(about one frame size).

IMPORTANT:
Changes have *no effect* on *already running* bakes.""",
        default=10.0,
        min=0.0,
        precision=2,
        options=set(),
    )  # type: ignore

    # ----------------------------------------------------------------
    # from_cache is read-only but can be overwritten with to_cache
    # ----------------------------------------------------------------
    from_cache: bpy.props.PointerProperty(
        type=Squishy_Volumes_Simulation_Settings,
        name="From Cache",
        description="The currently active set of settings (readonly).",
        options=set(),
    )  # type: ignore
    to_cache: bpy.props.PointerProperty(
        type=Squishy_Volumes_Simulation_Settings,
        name="To Cache",
        description="The modifiable set of settings.",
        options=set(),
    )  # type: ignore

    # ----------------------------------------------------------------
    # setup for capturing animation data
    # ----------------------------------------------------------------
    capture_start_frame: bpy.props.IntProperty(
        name="Capture Start Frame",
        description="""The start of the input objects' evaluation.

You need to override the cache to manifest changes.""",
        default=1,
        options=set(),
    )  # type: ignore
    capture_frames: bpy.props.IntProperty(
        name="Capture Frames",
        description="""The number of frames for which the input objects are evaluated.

You need to override the cache to manifest changes.""",
        default=250,
        min=1,
        options=set(),
    )  # type: ignore

    # ----------------------------------------------------------------
    # bake settings
    # ----------------------------------------------------------------
    immediately_start_baking: bpy.props.BoolProperty(
        name="Start Baking",
        description="""Save two mouse clicks!
If the setup is valid, overwriting the cache
starts baking with the current settings immediately.""",
        default=True,
        options=set(),
    )  # type: ignore
    time_step: bpy.props.FloatProperty(
        name="Time Step",
        description="""The discrete time resolution of the simulation.
Typically, many discrete steps are performed in between actual frames.

Can be tricky to get right: Lower values generally improve
stability and accuracy but impact performance.

Smaller grid node size, stiffer objects and higher velocities dictate a smaller timestep.

(Re)Start baking to manifest changes.""",
        default=0.01,
        min=0.000001,
        max=1.0,
        precision=5,
        options=set(),
    )  # type: ignore
    explicit: bpy.props.BoolProperty(
        name="Explicit",
        description="""TODO""",
        default=True,
        options=set(),
    )  # type: ignore
    debug_mode: bpy.props.BoolProperty(
        name="Debug Mode",
        description="""TODO""",
        default=False,
        options=set(),
    )  # type: ignore
    loaded_frame: bpy.props.IntProperty(
        name="Loaded Simulation Frame",
        description="""The index of the currently displayed simulation frame.
Baking can restart from it.""",
        default=-1,
        options=set(),
    )  # type: ignore
    bake_frames: bpy.props.IntProperty(
        name="Bake Frames",
        description="""The number of frames that should be baked.
Note that first frame is defined by the state at the set start frame.

(Re)Start baking to manifest changes.""",
        default=250,
        min=1,
        options=set(),
    )  # type: ignore

    # ----------------------------------------------------------------
    # display settings
    # ----------------------------------------------------------------
    display_start_frame: bpy.props.IntProperty(
        name="Start Diplaying at Frame",
        description="""When loading the simulated frames, start at this frame.
Usually the same as the start of the input objects' evaluation.

This only changes the display of simulation results and can be changed anytime.""",
        default=1,
        options=set(),
    )  # type: ignore
