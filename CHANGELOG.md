# [0.3.6] - 2026-09-10

With this release, Squishy Volumes switches from distributing Tree Clipper JSONs to Blender asset library files.
They are still created with Tree Clipper from the JSONs during the build.

This improves the overall user experience: node groups aren’t endlessly re-created, and we can use Blender 5.3.
(At least the current alpha version)

This release also includes several other smaller features and fixes.

## Features
- Shipping assets instead of JSONs
- Tentative support for Blender 5.3
- Global damping gives us a blunt, yet effective tool to deal with excessive jiggling.
- Compute stats stored with each frame.
- Basic CI checks
- Time markers are optional now.
- Icons to tell the type of input/output objects
- Ability to set the input_name of output objects, allowing for the recovery of desynced ones

## Changed
- Goals/Pinning-Feature is back to damped springs, which is more stable.
- Reject scaled input particle objects by default.

## Fixed
- Small-scale scenes allowed single particles to ignore friction.
- Avoid showing stack traces for errors meant to be seen by users.