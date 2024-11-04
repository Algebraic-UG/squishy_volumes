# Blended MPM

The Material Point Method (MPM) in Blender!

Here you can download ready-to-use releases, report bugs, and get the source code.

## Where to Get the Extension

You can either
- ❤ buy the extension ❤ (TODO: shop link)
- download ZIP from [github release page](https://github.com/Algebraic-UG/blended_mpm/releases)
- [build from source](#building)

## How to Use the Extension

### Install in Blender

Either drag & drop the extension ZIP file directly into Blender and enable it via the pop-up dialog.

Or from within Blender, click through these:

Edit -> Preferences -> Add-ons -> Top right arrow down (drop-down) -> Install from Disk -> select ZIP.

### Create Your First Simulation

TODO: Textual User Guide

TODO: Link Tutorial Video

### All the Features

TODO: Link to Book

TODO: Link Video Series

## Licensing

The Blender-specific Python extension code is licensed under [GPLv3](./LICENSE_GPLv3), while the Rust simulation code is licensed under [MIT](./LICENSE_MIT). To avoid confusion, a respective license header is included in each source file.

Third-party licenses can be found in the binary artifacts (Python wheels) included in the extension ZIP files.

## Building

> [!WARNING]
> Building is only documented to a certain extend.
> Ubuntu 24.04 is tested rigorously, and builds on other operating systems are automated in GitHub's workflows.
> The extension is not meant to be built by the average user.

That being said, you should be fine if you have experience with the involved tools and languages. We would love to hear from you about any issues you might have.
We are continually seeking ways to enhance the build system.

### Dev Requirements

- git
- rust ([rustup](https://rustup.rs/))
- python ([uv](https://github.com/astral-sh/uv))
- blender ([download](https://www.blender.org/download/))
- vscode ([download](https://code.visualstudio.com/))

### Clone the repository

```
git clone git@github.com:Algebraic-UG/blended_mpm.git
cd blended_mpm
```
> [!IMPORTANT]
> All subsequent steps assume the checkout directory as working directory.

### (Optional) Setup Python

This isn't technically needed for actual wheel building below since the `uvx` command takes care of the environment.
But it is needed for the `./rust` level `cargo build` command to succeed.
```
uv python install 3.11
uv python list 3.11
```
Copy the path and set the environment variable for PyO3.

For example:
```
export PYO3_PYTHON=/home/<user>/.local/share/uv/python/cpython-3.11.11-linux-x86_64-gnu/bin/python3.11
```

### Wrap

See also [rebuild_api.sh](./scripts/rebuild_api.sh).

Remove stale wheels
```
rm python/wheels/*
```
Build new wheel, omit `--no-default-features` for hot reloading.
```
cd rust/crates/wrap
uvx --python 3.11 maturin build --no-default-features --release --out ../../../python/wheels/
```
Update manifest to include new wheel.
```
cd -
uv run --with toml update_manifest.py
```

### Extension

This command produces a ready-to-install ZIP file.
```
cd python
blender --command extension build
```

You can then install it like [before](#install-in-blender).

### Hot Reload

TODO: describe how to hot reload while developing

> [!TIP]
> It's very similar to https://github.com/Algebraic-UG/blend_rust.

### Using VSCode for Python

See this helpful [video](https://www.youtube.com/watch?v=zP0s1i9EXeM).

1. Install VS Code.
2. Install Python, Python Debugger, and Blender Development.
3. Open the repo dir.
5. ctrl+shift+P -> Python: Create Environment -> Venv -> Enter interpreter path... -> Select Python that is bundled with Blender.
6. In the venv: `python -m pip install fake-bpy-module-latest`.
7. Restart VS Code.
8. ctrl+shift+P -> Blender: Build and Start -> Select Blender binary.
