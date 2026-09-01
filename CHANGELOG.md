# [0.3.5] - 2026-09-01

The most important change is that adaptive time steps are now available on the GPU.

## Features
- Adaptive time steps on GPU.
- Domain particle culling on the GPU.
- Load the latest state on failure, and indicate the failed particles.
- Hint to switch to object mode instead of showing nothing.

## Fixed
- CPU profiling was broken.
- Toggling pinning caused an issue.
- Allow particles to get squished much more before aborting.
- GPU collision logic was slightly different from the CPU version.
- The vertex normal calculation was too naive.
- There was an issue with querying particle flags.
