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

import os
from pathlib import Path
import bpy

from .bridge import load_simulation
from .util import simulation_cache_locked
from .frame_change import sync_simulation


class OBJECT_OT_Blended_MPM_Reload_All_Caches(bpy.types.Operator):
    bl_idname = "object.blended_mpm_reload_all_caches"
    bl_label = "Blended MPM Reload All Caches"
    bl_description = """Reloads all simulation caches.
This is useful when reloading a Blender filer with multiple simulations."""
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        for simulation in context.scene.blended_mpm_scene.simulations:
            lock_file = Path(simulation.cache_directory) / "lock"
            if os.path.exists(lock_file):
                os.remove(lock_file)
                self.report({"INFO"}, "Removed lock file.")

            simulation.last_exception = ""
            simulation.loaded_frame = -1
            load_simulation(simulation)
            sync_simulation(simulation, context.scene.frame_current)
            self.report({"INFO"}, "Reloaded simulation.")

        return {"FINISHED"}

    def invoke(self, context, _event):
        if any(
            [
                simulation_cache_locked(simulation)
                for simulation in context.scene.blended_mpm_scene.simulations
            ]
        ):
            return context.window_manager.invoke_props_dialog(self)
        else:
            return self.execute(context)

    def draw(self, context):
        self.layout.label(text="WARNING: these caches contain lock files:")
        for name in [
            simulation.name
            for simulation in context.scene.blended_mpm_scene.simulations
            if simulation_cache_locked(simulation)
        ]:
            self.layout.lablel(text=f"{name}")
        self.layout.lablel(text="Confirm to remove them.")


classes = [
    OBJECT_OT_Blended_MPM_Reload_All_Caches,
]


def menu_func_reload_all(self, _context):
    self.layout.operator(
        OBJECT_OT_Blended_MPM_Reload_All_Caches.bl_idname, icon="FILE_CACHE"
    )


menu_funcs = [menu_func_reload_all]


def register_reload_utils():
    for cls in classes:
        bpy.utils.register_class(cls)
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.append(menu_func)


def unregister_reload_utils():
    for menu_func in menu_funcs:
        bpy.types.VIEW3D_MT_object.remove(menu_func)
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
