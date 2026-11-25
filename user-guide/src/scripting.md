# Scripting

Simulations can also be set up and baked from Python.
This works in Blender's Python console and Text Editor as well as headless with `blender -b`.

## Import

While Squishy Volumes is enabled, its scripting API can be imported as `squishy_volumes`:

```python
import squishy_volumes as sv
```

`sv.__all__` lists everything the API offers, and `help(sv.add_input)` shows the parameters of a single function.
Problems are reported by raising a subclass of `sv.SquishyVolumesError`.

## Minimal Script

This script recreates the squishy cube from the [Example Setup](getting_started/example_setup.md), records the input and bakes ten frames:

```python
import bpy
import squishy_volumes as sv

sim_obj = sv.add_simulation("Jelly")

bpy.ops.mesh.primitive_cube_add(size=2)
sv.add_input(sim_obj, bpy.context.active_object, sv.PARTICLES)

bpy.ops.mesh.primitive_plane_add(size=10, location=(0, 0, -2))
sv.add_input(sim_obj, bpy.context.active_object, sv.COLLIDER)

sv.record_input(sim_obj)
sv.bake(sim_obj, bake_frames=10)
sv.wait_until_baked(sim_obj, timeout_sec=600)

bpy.ops.wm.save_mainfile()
```

The default cache location is relative to the `.blend` file, so save the file before running the script,
or pass an absolute `directory` to `sv.add_simulation`.
To run it without the UI:

```sh
blender -b jelly.blend --python jelly.py
```

Without `wait_until_baked`, the bake keeps running in the background, just like when started from the panels.

## More Examples

The [example setups](https://github.com/Algebraic-UG/squishy_volumes/tree/main/python/src/squishy_volumes_extension/example) behind the 'Example Setup' button are written with the same API.
They also show how to change simulation settings and the output visualization.
