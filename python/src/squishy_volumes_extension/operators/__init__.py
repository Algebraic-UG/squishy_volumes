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

from .drivers import register_drivers, unregister_drivers
from .goals import register_goals, unregister_goals
from .input_output import (
    register_input_output_operators,
    unregister_input_output_operators,
)
from .overview import register_overview_operators, unregister_overview_operators
from .popup import register_popup, unregister_popup
from .render_setup import register_render_setup, unregister_render_setup
from .simulate import register_simulate_operators, unregister_simulate_operators
from .view_utils import register_view_utils, unregister_view_utils


def register_operators():
    register_popup()
    register_overview_operators()
    register_input_output_operators()
    register_simulate_operators()
    register_goals()
    register_view_utils()
    register_render_setup()
    register_drivers()


def unregister_operators():
    unregister_drivers()
    unregister_render_setup()
    unregister_view_utils()
    unregister_goals()
    unregister_simulate_operators()
    unregister_input_output_operators()
    unregister_overview_operators()
    unregister_popup()
