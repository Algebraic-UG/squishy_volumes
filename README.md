<p align="center">
<img src="logo_with_text.svg" alt="The Squishy Volumes logo">
</p>

<center>
    <div class="row">
        <img alt="GitHub Release" src="https://img.shields.io/github/v/release/Algebraic-UG/squishy_volumes">
        <a href="https://discord.gg/P8mrnQEavf">
        <img alt="Discord" src="https://img.shields.io/discord/1336035911766511687?logo=discord">
        </a>
        <a href="https://algebraicug.gumroad.com/l/squishy_volumes">
        <img alt="Static Badge" src="https://img.shields.io/badge/Gumroad-%E2%9D%A4%EF%B8%8F-blue?logo=gumroad">
        </a>
    </div>
</center>

# Squishy Volumes

The Material Point Method (MPM) in Blender!

Here you can download ready-to-use releases, report bugs, and get the source code.

Works in Blender 5.0, 5.1, and 5.2.

## Where to Get the Extension

You can either
- [❤ buy the extension ❤](https://algebraicug.gumroad.com/l/squishy_volumes)
- download ZIP from [github release page](https://github.com/Algebraic-UG/squishy_volumes/releases)
- [build from source](#building)

Release ZIPs carry [GitHub artifact attestations](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations).
To check that a downloaded ZIP was built by this repository's CI, run:

```bash
gh attestation verify squishy_volumes_extension-<version>-<platform>.zip --repo Algebraic-UG/squishy_volumes
```

## How to Use the Extension

Please refer to the [Squishy Volumes User Guide](https://squishy-volumes-docs.algebraic.games/index.html).

## Licensing

Squishy Volumes is licensed under the [GNU Affero General Public License v3.0 or later](./LICENSE) (AGPL-3.0-or-later). This covers both the Blender-specific Python extension code and the Rust simulation code. Each source file carries a respective license header.

For commercial licensing under different terms, please contact Algebraic UG (haftungsbeschränkt).

Third-party licenses can be found in the binary artifacts (Python wheels) included in the extension ZIP files.

## Building

> [!WARNING]
> Building is only documented to a certain extent.
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
git clone git@github.com:Algebraic-UG/squishy_volumes.git
cd squishy_volumes
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
rm python/src/squishy_volumes_extension/wheels/*
```
Build new wheel, omit `--no-default-features` for hot reloading.
```
cd rust/crates/wrap
uvx --python 3.11 maturin build --no-default-features --release --out ../../../python/src/squishy_volumes_extension/wheels/
```
Update manifest to include new wheel.
```
cd -
uv run --with toml scripts/update_manifest.py
```

### Extension

This command produces a ready-to-install ZIP file.
```
cd python/src/squishy_volumes_extension
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

## Automatic Tests

The `step` tests is currently broken.
See https://github.com/Algebraic-UG/squishy_volumes/issues/190

```
cd rust
cargo test --release
```
