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

# The code in this module has been mostly generated with https://github.com/BrendanParmer/NodeToPython
# NodeToPython is licensed under the GPLv3 License.

# XXX: reminder for maintainer!
#
# place code inside the function and return the top group
#
# change the fixed attribute strings to python constants use this
#
#     %s/\"squishy_volumes_([^"]*)\"/SQUISHY_VOLUMES_\U\1/gc
#
# any custom node groups referenced through strings must be recreated and not looked up
# (search for "Squishy Volumes <...>" lookups)
