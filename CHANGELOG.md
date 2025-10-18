# Change Log

## [1.1.19] - 2025-10-18

The prominent feature for this release is a *much* improved heuristic for safe time steps.
For most scenes, we expect the time step size to require no further tuning.
Expect a (slight) performance regression when it's activated.
It's possible to opt out.

### Features
- Time step heuristic
- Add multiple outputs at once
- Simulation stats and remaining time estimate
- Material coordinates in default mesh reconstruction
- Improved error messages

### Fixed Bugs
- The tutorial logic was looking at the wrong settings

### Performance
- A Regression due to unoptimized code in the time step heuristic