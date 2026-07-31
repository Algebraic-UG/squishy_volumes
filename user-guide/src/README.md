<center><img src="logo_with_text.svg" alt="The Squishy Volumes logo"></center>

# Introduction

This is the official Squishy Volumes 0.3.2 user guide.

You're likely looking for [Getting Started](./getting_started/index.html).

## What is Squishy Volumes?

Squishy Volumes is an [open source](https://github.com/Algebraic-UG/squishy_volumes) implementation of the [Material Point Method (MPM)](https://en.wikipedia.org/wiki/Material_point_method), developed by [Algebraic](https://algebraic.games/), and available as a [Blender](https://www.blender.org/) extension.

> [!IMPORTANT]
> The Squishy Volumes extension currently requires Blender 5.0, 5.1, or 5.2.

Squishy Volumes is not implemented in Blender's [Geometry Nodes](https://docs.blender.org/manual/en/latest/modeling/geometry_nodes/index.html). It is written in a combination of [Python](https://www.python.org/) for the Blender integration, [Rust](https://rust-lang.org/) for the core logic, and [WGSL](https://www.w3.org/TR/WGSL/) for the compute shaders.

Geometry Nodes are used for customizable input generation and output visualization.

<u>Squishy Volumes is GPU-accelerated!</u>

You can run Squishy Volumes on:
- Linux
- Windows
- MacOS

And all major GPUs are supported:
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
