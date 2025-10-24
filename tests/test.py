# suzanne_render.py
import bpy
import os
from mathutils import Vector

# --- 1) Remove the default cube (if present) ---
cube = bpy.data.objects.get("Cube")
if cube:
    bpy.data.objects.remove(cube, do_unlink=True)

# --- 2) Ensure there's at least one light ---
if not any(obj.type == "LIGHT" for obj in bpy.context.scene.objects):
    bpy.ops.object.light_add(type="SUN", location=(5, -5, 5))

# --- 3) Add Suzanne at the origin and smooth shade ---
bpy.ops.mesh.primitive_monkey_add(size=1.5, location=(0, 0, 0))
monkey = bpy.context.active_object
monkey.name = "Suzanne"
bpy.ops.object.shade_smooth()

# --- 4) Make the camera look at Suzanne (works even if you moved the camera) ---
cam = bpy.data.objects.get("Camera")
tracking_constraint = cam.constraints.new(type="TRACK_TO")
tracking_constraint.target = monkey

# --- 5) Render to the current directory as PNG ---
scene = bpy.context.scene
scene.render.image_settings.file_format = "PNG"
scene.render.filepath = os.path.join(os.getcwd(), "suzanne.png")

bpy.ops.render.render(write_still=True)
print("Rendered:", scene.render.filepath)
