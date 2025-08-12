import bpy

from ..progress_update import is_updating, register_progress_update


class OBJECT_OT_Blended_MPM_Restart_Update_Loop(bpy.types.Operator):
    bl_idname = "object.blended_mpm_restart_update_loop"
    bl_label = "Restart Update Loop"
    bl_options = {"REGISTER"}

    def execute(self, _context):
        register_progress_update()
        return {"FINISHED"}


class OBJECT_PT_Blended_MPM_Restart_Crashed_Loop(bpy.types.Panel):
    bl_label = "Restart Crashed Loop"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "Blended MPM"

    @classmethod
    def poll(cls, context):
        return context.mode == "OBJECT" and not is_updating()

    def draw(self, _context):
        self.layout.label(text="If you see this, the update loop has failed.")
        self.layout.label(text="This is an unexpected error.")

        self.layout.alert = True
        self.layout.operator(
            OBJECT_OT_Blended_MPM_Restart_Update_Loop.bl_idname, icon="FILE_REFRESH"
        )


classes = [
    OBJECT_OT_Blended_MPM_Restart_Update_Loop,
    OBJECT_PT_Blended_MPM_Restart_Crashed_Loop,
]


def register_panel_restart_update():
    for cls in classes:
        bpy.utils.register_class(cls)


def unregister_panel_restart_update():
    for cls in reversed(classes):
        bpy.utils.unregister_class(cls)
