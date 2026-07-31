<center><img src="logo_with_text.svg" alt="The Squishy Volumes logo"></center>

# Introduction

This is the official Squishy Volumes 0.3.2 user guide.

You are likely looking for [Getting Started](./getting_started/index.html).

## What is Squishy Volumes?

Squishy Volumes is an [open source](https://github.com/Algebraic-UG/squishy_volumes) implementation of the [Material Point Method (MPM)](https://en.wikipedia.org/wiki/Material_point_method), developed by [Algebraic](https://algebraic.games/), and available as a [Blender](https://www.blender.org/) extension.

> [!IMPORTANT]
> The Squishy Volumes extension currently requires Blender 5.0, 5.1, or 5.2.

Squishy Volumes is physically based and allows for realistic simulations of a wide range of materials.
It is written in a combination of [Python](https://www.python.org/) for the Blender integration, [Rust](https://rust-lang.org/) for the core logic, and [WGSL](https://www.w3.org/TR/WGSL/) for the compute shaders.
Squishy Volumes utilizes [Geometry Nodes](https://docs.blender.org/manual/en/latest/modeling/geometry_nodes/index.html) for customizable input generation and output visualization.

You can run Squishy Volumes on:
- Linux
- Windows
- macOS

Squishy Volumes can use your GPU! It runs <u>significantly faster</u> compared to the CPU, and all major GPU vendors are supported:
- NVIDIA
- AMD
- Apple Silicon

## Showcase

<iframe
  align="middle"
  width="100%"
  height="420"
  allowfullscreen
  style="display:block;margin:auto;"
  src="https://www.youtube.com/embed/BH0PTvSxFx8">
</iframe>
