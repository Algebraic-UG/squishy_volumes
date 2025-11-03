# Change Log
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/)
and this project adheres to [Semantic Versioning](http://semver.org/).

# Change Log

# Change Log

## [0.1.21] - 2025-11-04

Fix refactoring error and add default visualization to fluids.

### Features
- Default fluid visualization

### Fixes
- Broken fluid output

## [0.1.20] - 2025-11-03

We're getting close to automated testing and general scripting from within Blender. Known blockers for Blender 5 support are gone, and we have a simple domain check.

### Features
- Local testing
- Blender 5 tentative support
- AABB domain to catch escaping particles
- Multi cache reload
- Streamlined adding simulations

### Performance
- Parallel time step heuristic

## [0.1.19] - 2025-10-18

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

## [0.1.18] - 2025-10-02

With this release we'll have a much smoother update!

A 'renaming trick' allows for installing a new version without closing Blender
and the release files now contain an `index.json` which allows checking for updates!

## [0.1.17] - 2025-09-12

The extension got a new name: "Squishy Volumes"!
The main features include a rudimentary sand simulation and significantly improved mesh advection.
The most critical fixed bug was that friction was not consistent across different collider locations.

## [0.1.16] - 2025-08-29

A whole bunch of bug fixes and two notable features: simulation scaling and viscosity!

## [0.1.15] - 2025-08-08

This release includes a new in-UI tutorial, several minor features, and significant performance improvements!

## [0.1.14] - 2025-07-28

Fix slow serialization.

## [0.1.13] - 2025-07-23

Initial release.

### Added

### Changed

### Fixed
