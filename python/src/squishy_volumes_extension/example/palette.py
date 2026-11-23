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

PARTICLE_COLORS = (
    "#D9825B",
    "#E3B45F",
    "#8DB07B",
    "#5FA8A0",
    "#6E8FC7",
    "#9C83C4",
    "#C97B9B",
    "#C9A27E",
)
COLLIDER_COLORS = (
    "#8A8F94",
    "#A39E93",
    "#6F7F88",
    "#B8B2A6",
)


def particle_material(index: int) -> bpy.types.Material:
    return _palette_material("Particles", PARTICLE_COLORS, index)


def collider_material(index: int) -> bpy.types.Material:
    return _palette_material("Collider", COLLIDER_COLORS, index)


def assign_material(obj: bpy.types.Object, material: bpy.types.Material):
    if not obj.material_slots:
        obj.data.materials.append(None)
    slot = obj.material_slots[0]
    slot.link = "OBJECT"
    slot.material = material


def linear_color(hex_color: str) -> tuple[float, float, float, float]:
    channels = [int(hex_color[i : i + 2], 16) / 255 for i in (1, 3, 5)]
    return (*(_srgb_to_linear(channel) for channel in channels), 1.0)


def _palette_material(
    group: str, colors: tuple[str, ...], index: int
) -> bpy.types.Material:
    name = f"Squishy Volumes {group} {index + 1}"
    material = bpy.data.materials.get(name)
    if material is not None:
        return material
    material = bpy.data.materials.new(name)
    color = linear_color(colors[index % len(colors)])
    material.diffuse_color = color
    material.use_nodes = True
    material.node_tree.nodes["Principled BSDF"].inputs[
        "Base Color"
    ].default_value = color
    return material


def _srgb_to_linear(channel: float) -> float:
    if channel <= 0.04045:
        return channel / 12.92
    return ((channel + 0.055) / 1.055) ** 2.4
