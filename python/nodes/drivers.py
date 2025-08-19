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

import re
import bpy

from ..util import get_simulation_idx_by_uuid


def add_drivers(simulation, modifier):
    simulation_idx = get_simulation_idx_by_uuid(simulation.uuid)
    tree = modifier.node_group.interface.items_tree
    if "Grid Node Size" in tree:
        identifier = tree["Grid Node Size"].identifier
        driver = modifier.driver_add(f'["{identifier}"]').driver
        driver.expression = "grid_node_size"
        var = driver.variables.new()
        var.name = "grid_node_size"
        var.type = "CONTEXT_PROP"
        var.targets[0].data_path = (
            f"blended_mpm_scene.simulations[{simulation_idx}].from_cache.grid_node_size"
        )

    if "Particle Size" in tree:
        identifier = tree["Particle Size"].identifier
        driver = modifier.driver_add(f'["{identifier}"]').driver
        driver.expression = "particle_size"
        var = driver.variables.new()
        var.name = "particle_size"
        var.type = "CONTEXT_PROP"
        var.targets[0].data_path = (
            f"blended_mpm_scene.simulations[{simulation_idx}].from_cache.particle_size"
        )


DRIVER_PATTERN = r"^(blended_mpm_scene\.simulations\[)(\d+)(\].*)"


def remove_drivers(obj):
    drivers_to_remove = []
    if not obj.animation_data:
        return
    for driver in obj.animation_data.drivers:
        for variable in driver.driver.variables:
            for target in variable.targets:
                re_match = re.match(DRIVER_PATTERN, target.data_path)
                if re_match:
                    drivers_to_remove.append(driver)
                    break
            else:
                continue
            break
    for driver in drivers_to_remove:
        obj.animation_data.drivers.remove(driver)


def update_drivers(removed_simulation_idx):
    for obj in bpy.data.objects:
        drivers_to_remove = []
        if not obj.animation_data:
            continue
        for driver in obj.animation_data.drivers:
            for variable in driver.driver.variables:
                for target in variable.targets:
                    re_match = re.match(DRIVER_PATTERN, target.data_path)
                    if re_match:
                        prior_simulation_idx = int(re_match.group(2))
                        if removed_simulation_idx == prior_simulation_idx:
                            drivers_to_remove.append(driver)
                            break
                        if removed_simulation_idx < prior_simulation_idx:
                            target.data_path = (
                                re_match.group(1)
                                + str(prior_simulation_idx - 1)
                                + re_match.group(3)
                            )
                else:
                    continue
                break
        for driver in drivers_to_remove:
            obj.animation_data.drivers.remove(driver)
