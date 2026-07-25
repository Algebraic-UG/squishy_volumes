# [0.3.0] - 2026-07-24

This release adds *GPU support* to Squishy Volumes.
Depending on the scene and hardware, this yields more than *30x speedup*!

For example, the [desert space ship scene](https://algebraic.games/blog/desert_space_ship/) took more than 18 hours to simulate before, but now can be simulated in less than 35 minutes.

While GPU support is the most prominent feature, [the entire codebase has been overhauled](https://github.com/Algebraic-UG/squishy_volumes/compare/v0.2.0...v0.3.0).

Consider getting Squishy Volumes via [Gumroad](https://algebraicug.gumroad.com/l/squishy_volumes).

## Features

- GPU support, yielding significant simulation speedup
  - NVIDIA, AMD, and Apple Silicon
  - Up to 20 million particles
- Example Scenes

## Fixes

- Playback state is restored after input recording
- Undo cannot cause an orphaned simulation handle
- Input validation errors contain object name
- Friction is more consistent due to new collision logic

## Changed

- New collision logic:
  - Resolution independent
  - Significantly faster for large colliders
  - Limited to 16 separate collider objects
  - Removes special grid outputs
  - Default visualization Geometry Nodes updated
- UI changes:
  - Add simulation buttons on top
  - Renames:
    - ‘Bake’ panel is now ‘Simulate’ panel
    - ‘Initialize/Overwrite Cache’ is now ‘Record Input’
  - Input recording controls are in the simulate panel
  - Output panel is now between input and simulate
  - Add buttons are alerted if no input or output exists
- Under the hood:
  - Rust code split into more crates
  - anyhow replaced with thiserror
  - Python code restructured to standard uv project layout
  - Tree Clipper updated to 0.1.18 as prep. for Blender 5.2 support