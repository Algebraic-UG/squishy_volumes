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

from .benchmark import EXAMPLE_BENCHMARK, setup_example_benchmark
from .boing_block import EXAMPLE_BOING_BLOCK, setup_example_boing_block


def setup_example_simulation(context, choice):
    if choice == EXAMPLE_BOING_BLOCK:
        return setup_example_boing_block(context)
    if choice == EXAMPLE_BENCHMARK:
        return setup_example_benchmark(context)
    raise RuntimeError(f"Unknown example simulation: {choice}")
