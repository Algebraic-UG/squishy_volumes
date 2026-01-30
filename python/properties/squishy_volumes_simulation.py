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
    simulations = bpy.context.scene.squishy_volumes_scene.simulations  # ty:ignore[unresolved-attribute]
    return any(
        [
            simulation.name == other.name
            for other in simulations
            if other.uuid != simulation.uuid
        ]
    )


def duplicate_simulation_cache_directory(simulation):
    simulations = bpy.context.scene.squishy_volumes_scene.simulations  # ty:ignore[unresolved-attribute]
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
        description="It is just the name without any semantic implications.",
        default="My Simulation",
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
        subtype="DIR_PATH",
        options=set(),
        update=update_cache_directory,
    )  # type: ignore

    sync: bpy.props.BoolProperty(
        name="Sync",
        description="""Disable to stop Squishy Volumes from loading and syncing.

For large scenes, frame changes are expensive even if no output is present.
It is then convenient to temporarily disable syncing for the simulation.""",
        default=True,
        options=set(),
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
    # These are constant
    # ----------------------------------------------------------------
    grid_node_size: bpy.props.FloatProperty(
        name="Grid Node Size",
        description="""The major discrete space resolution of the simulation.
Can be tricky to get right: Lower values grant higher fidelity
in the simulation but impacts performance and stability.

Overwrite the cache to manifest changes.""",
        default=0.5,
        min=0.001,
        precision=5,
        options=set(),  # can't be animated
    )  # type: ignore
    frames_per_second: bpy.props.IntProperty(
        name="Frames per Second",
        description="""Controls how many simulation steps end up as viewable frames per simulated second.
If blender's native FPS differs from this setting you'll get 'artifical' speedup or slowdown.

For example:
Given that blender's native FPS is set to 24 (default),
to achieve a 4x 'slowmotion' effect, you need to set this to 96.

Note that this also effects the interpretation of captured animations from input objects.

Overwrite the cache to manifest changes.""",
        default=24,
        min=1,
        options=set(),  # can't be animated
    )  # type: ignore
    simulation_scale: bpy.props.FloatProperty(
        name="Simulation Scale",
        description="""Use this to simulate things as if they were bigger or smaller.

For example, if your scene is 10 meters long but should behave as if it were 10 centimeters,
you can set this to 100.""",
        default=1.0,
        min=0.001,
        max=1000.0,
        precision=6,
        options=set(),  # can't be animated
    )  # type: ignore
    domain_min: bpy.props.FloatVectorProperty(
        name="Domain Min",
        description="""The min corner of the domain AABB.
Particles that fall below this are deactivated.""",
        default=(-100.0, -100.0, -100.0),
        options=set(),  # can't be animated
    )  # type: ignore
    domain_max: bpy.props.FloatVectorProperty(
        name="Domain Max",
        description="""The max corner of the domain AABB
Particles that rise above this are deactivated.""",
        default=(100.0, 100.0, 100.0),
        options=set(),  # can't be animated
    )  # type: ignore

    # ----------------------------------------------------------------
    # These can be animated over time
    # ----------------------------------------------------------------
    gravity: bpy.props.FloatVectorProperty(
        name="Gravity",
        description="It is currently the only volumetric force and it is constant.",
        default=(0.0, 0.0, -9.8),
        options={"ANIMATABLE"},
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
    adaptive_time_steps: bpy.props.BoolProperty(
        name="Adaptive Time Steps",
        description="""Automatically determine a good 'Time Step'.
The manually set value is chosen if it is smaller than
the automatic one.""",
        default=True,
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
