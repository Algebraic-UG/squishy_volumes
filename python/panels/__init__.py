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

from .panel_overview import register_panel_overview, unregister_panel_overview
from .panel_input import register_panel_input, unregister_panel_input
from .panel_bake import register_panel_bake, unregister_panel_bake
from .panel_output import register_panel_output, unregister_panel_output


def register_panels():
    register_panel_overview()
    register_panel_input()
    register_panel_bake()
    register_panel_output()
    print("Squishy Volumes panels registered.")


def unregister_panels():
    unregister_panel_output()
    unregister_panel_bake()
    unregister_panel_input()
    unregister_panel_overview()
    print("Squishy Volumes panels unregistered.")
