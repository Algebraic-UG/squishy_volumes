# SPDX-License-Identifier: AGPL-3.0-or-later
#
# This file is part of the Squishy Volumes extension.
# Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published
# by the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.

import bpy

from .bridge import DETECTED_DEVICES
from .registration import class_registration


def get_default_compute_device_index(preferences) -> int:
    detected_identifiers = [device[0] for device in DETECTED_DEVICES]
    identifier = preferences.default_compute_device_identifier
    if identifier in detected_identifiers:
        return detected_identifiers.index(identifier)
    return 0


def set_default_compute_device_index(preferences, index: int) -> None:
    preferences.default_compute_device_identifier = DETECTED_DEVICES[index][0]


class SquishyVolumesPreferences(bpy.types.AddonPreferences):
    bl_idname = __package__

    default_cache_location: bpy.props.StringProperty(
        name="Default Cache",
        description="""Default directory that holds the relevant simulation data.
Paths starting with '//' are relative to the .blend file.
It's a good idea to point this to a separate disk from the operating system.""",
        default="//squishy_volumes_cache",
        subtype="DIR_PATH",
        options={"PATH_SUPPORTS_BLEND_RELATIVE"},
    )

    sanity_check_allowed_disk_space: bpy.props.BoolProperty(
        name="Sanity Check Max Disk Space",
        description="""Enable/Disable the sanity check for allowed disk space.

When a bake starts and before each frame is stored, the remaining allowed
disk space (max minus what the cache already uses) is compared to
the currently free space on the disk where the cache is located.
If too little is free, baking stops with an error.

Changes take effect the next time a bake starts.""",
        default=True,
        options=set(),
    )

    confirm_bake_overwrite: bpy.props.BoolProperty(
        name="Confirm Overwrite",
        description="""Each time the simulation input is (over)written, old frames are discarded.

To prevent accidental deletion a popup adds a manual confirmation step:
'WARNING: This is a destructive operation!'

Disable this option to skip that popup.""",
        default=False,
    )

    domain_min: bpy.props.FloatVectorProperty(
        name="Domain Min",
        description="""The min corner of the domain AABB.
Particles that fall below this are deactivated.""",
        default=(-100.0, -100.0, -100.0),
        options=set(),  # can't be animated
    )
    domain_max: bpy.props.FloatVectorProperty(
        name="Domain Max",
        description="""The max corner of the domain AABB
Particles that rise above this are deactivated.""",
        default=(100.0, 100.0, 100.0),
        options=set(),  # can't be animated
    )

    print_debug_info: bpy.props.BoolProperty(
        name="Print Debug Info",
        description="""Can be used to disable certain debug printouts.
This is most likely only relevant to developers of other extensions.""",
        default=True,
        options=set(),  # can't be animated
    )

    enable_timeline_markers: bpy.props.BoolProperty(
        name="Enable Timeline Markers",
        description="""Use timeline markers to set the frames to record.

When enabled, each simulation has two timeline markers named like it.
Recording evaluates the input objects from the first to the last of them.

When disabled, no markers are created, read or removed,
and the frames to record are set in the 'Simulate' panel instead.""",
        default=True,
        options=set(),  # can't be animated
    )

    allow_scaled_input: bpy.props.BoolProperty(
        name="Allow Scaled Input",
        description="""Squishy Volumes to accept scaled input particle objects.

The default input generation uses the object scale
to determine the initial compression/stretching of the material.

This is a useful feature, but it is easy to 'misuse' by accident,
which is why any scaling is rejected by default.""",
        default=False,
        options=set(),  # can't be animated
    )

    default_compute_device_identifier: bpy.props.StringProperty(
        options={"HIDDEN"},
    )

    default_compute_device: bpy.props.EnumProperty(
        items=DETECTED_DEVICES,
        name="Default Compute Device",
        description="""The device that newly created simulations use.

Existing simulations keep their device.
If this device isn't detected anymore, the first detected device is used.""",
        get=get_default_compute_device_index,
        set=set_default_compute_device_index,
        options=set(),
    )

    first_run_setup_done: bpy.props.BoolProperty(
        name="First Run Setup Done",
        description="""Hides the first run setup in the Squishy Volumes sidebar.
Disable it to show the setup again.""",
        default=False,
        options=set(),
    )

    def draw(self, context: bpy.types.Context) -> None:
        self.layout.prop(self, "default_cache_location")
        self.layout.prop(self, "default_compute_device")
        self.layout.prop(self, "sanity_check_allowed_disk_space")
        self.layout.prop(self, "confirm_bake_overwrite")
        self.layout.prop(self, "domain_min")
        self.layout.prop(self, "domain_max")
        self.layout.prop(self, "print_debug_info")
        self.layout.prop(self, "enable_timeline_markers")
        self.layout.prop(self, "allow_scaled_input")
        self.layout.prop(self, "first_run_setup_done")


register_preferences, unregister_preferences = class_registration(
    [SquishyVolumesPreferences]
)
