# Change Log

## [0.2.0-alpha] - 2026-02-22

> [!NOTE]
> This is an *alpha* release.
> Performance is not on-par with 0.1.21 for colliders and certain features are missing:
> The simulation scale does nothing and skinning is not in yet.

This release is *packed*!

The overarching change here is the way we can define inputs for Squishy Volumes.

Before, all the input was contained in a JSON file, which was very slow to load for big meshes and animated meshes were completely out of the question.
Now, there is a dedicated binary format.
This allows us to capture much more stuff and we can leverage Geometry Nodes to create it.

In essence, more control and faster feedback.

### Features
- [Binary input!](https://github.com/Algebraic-UG/squishy_volumes/issues/20)
- Responsive input generation in Blender and per-particle physical parameters
- Animated goal positions, aka. Pinning
- Deformable Colliders
- Bulk viscosity
- [Optional sync](https://github.com/Algebraic-UG/squishy_volumes/pull/145)
- Animatable gravity
- Opt-out for default visualization

### Fixes
- More sensible viscosity values ([still not perfect](https://github.com/Algebraic-UG/squishy_volumes/issues/102)) 
- Explicit initial state creation [bug](https://github.com/Algebraic-UG/squishy_volumes/issues/140)
- Collider grid [pruning](https://github.com/Algebraic-UG/squishy_volumes/issues/10)
- Input/Output object [selection sync](https://github.com/Algebraic-UG/squishy_volumes/issues/128) with BlenderP
- Setup creation runs modal ([doesn't block UI anymore](https://github.com/Algebraic-UG/squishy_volumes/issues/135))

### Changed
- All the nodes are now loaded with [Tree Clipper](https://github.com/Algebraic-UG/tree_clipper)

### Performance

The performance of the colliders is expected to be worse for the alpha release.
