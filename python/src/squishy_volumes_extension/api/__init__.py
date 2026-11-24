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

from ..errors import (
    BakeFailed,
    BakeTimeout,
    CaptureRangeError,
    InsufficientDiskSpace,
    InvalidBakeRange,
    InvalidInput,
    InvalidRenderSetup,
    ScaledInputError,
    SimulationNotFound,
    SimulationNotLoaded,
    SquishyVolumesError,
    UnresolvableCacheDirectory,
)
from .bake import (
    bake,
    bake_errors,
    baked_frames,
    clear_bake_errors,
    is_baking,
    pause_bake,
    wait_until_baked,
)
from .inputs import (
    COLLIDER,
    PARTICLES,
    add_input,
    copy_alembic_attributes,
    input_problem,
    is_input,
    remove_input,
    reset_alembic_attributes,
)
from .record import (
    capture_range,
    iter_record_input,
    record_input,
    set_capture_range,
)
from .render import (
    PREVIEW,
    RENDER_PRESETS,
    SHOWCASE,
    render_frames,
    set_up_render,
)
from .simulation import (
    add_simulation,
    clone_simulation,
    find_simulation,
    input_objects,
    reload,
    reload_all,
    remove_simulation,
    simulations,
)
from .tools import (
    add_drivers,
    add_goals,
    restrict_view,
    set_modifier_input,
)

__all__ = [
    "COLLIDER",
    "PARTICLES",
    "PREVIEW",
    "RENDER_PRESETS",
    "SHOWCASE",
    "BakeFailed",
    "BakeTimeout",
    "CaptureRangeError",
    "InsufficientDiskSpace",
    "InvalidBakeRange",
    "InvalidInput",
    "InvalidRenderSetup",
    "ScaledInputError",
    "SimulationNotFound",
    "SimulationNotLoaded",
    "SquishyVolumesError",
    "UnresolvableCacheDirectory",
    "add_drivers",
    "add_goals",
    "add_input",
    "add_simulation",
    "bake",
    "bake_errors",
    "baked_frames",
    "capture_range",
    "clear_bake_errors",
    "clone_simulation",
    "copy_alembic_attributes",
    "find_simulation",
    "input_objects",
    "input_problem",
    "is_baking",
    "is_input",
    "iter_record_input",
    "pause_bake",
    "record_input",
    "reload",
    "reload_all",
    "remove_input",
    "remove_simulation",
    "render_frames",
    "reset_alembic_attributes",
    "restrict_view",
    "set_capture_range",
    "set_modifier_input",
    "set_up_render",
    "simulations",
    "wait_until_baked",
]
