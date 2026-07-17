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


def add_drivers(sim_obj, modifier):
    tree = modifier.node_group.interface.items_tree
    if "Grid Node Size" in tree:
        identifier = tree["Grid Node Size"].identifier
        driver = modifier.driver_add(f'["{identifier}"]').driver
        driver.expression = "grid_node_size"
        var = driver.variables.new()
        var.name = "grid_node_size"
        var.type = "SINGLE_PROP"
        target = var.targets[0]
        target.fallback_value = 1
        target.data_path = "squishy_volumes.grid_node_size"
        target.id_type = "OBJECT"
        target.id = sim_obj
