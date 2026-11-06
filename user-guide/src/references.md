# References

Squishy Volumes builds on the work of others.
This page lists the external sources that the implementation follows or is derived from:
publications, articles and blog posts, and code and algorithms.
Ordinary library dependencies are not listed here.

## Publications

### Material Point Method

- Alexey Stomakhin, Craig Schroeder, Lawrence Chai, Joseph Teran, Andrew Selle.
  **A Material Point Method for Snow Simulation.**
  ACM Transactions on Graphics 32(4), SIGGRAPH 2013.
  [doi:10.1145/2461912.2461948](https://doi.org/10.1145/2461912.2461948)

  Introduced MPM to computer graphics; the particle-grid scheme the simulation builds on.

- Alexey Stomakhin, Craig Schroeder, Chenfanfu Jiang, Lawrence Chai, Joseph Teran, Andrew Selle.
  **Augmented MPM for Phase-Change and Varied Materials.**
  ACM Transactions on Graphics 33(4), SIGGRAPH 2014.
  [doi:10.1145/2601097.2601176](https://doi.org/10.1145/2601097.2601176)

  Treating fluids through their volume change alone, discarding the deviatoric part of the deformation.

- Chenfanfu Jiang, Craig Schroeder, Joseph Teran, Alexey Stomakhin, Andrew Selle.
  **The Material Point Method for Simulating Continuum Materials.**
  ACM SIGGRAPH 2016 Courses.
  [doi:10.1145/2897826.2927348](https://doi.org/10.1145/2897826.2927348)

  The overall MPM scheme, the quadratic B-spline weights, and the neo-Hookean stress.

- Chenfanfu Jiang, Craig Schroeder, Andrew Selle, Joseph Teran, Alexey Stomakhin.
  **The Affine Particle-In-Cell Method.**
  ACM Transactions on Graphics 34(4), SIGGRAPH 2015.
  [doi:10.1145/2766996](https://doi.org/10.1145/2766996)

  The affine velocity field carried by each particle in the particle-grid transfers.

- Yuanming Hu, Yu Fang, Ziheng Ge, Ziyin Qu, Yixin Zhu, Andre Pradhana, Chenfanfu Jiang.
  **A Moving Least Squares Material Point Method with Displacement Discontinuity and Two-Way Rigid Body Coupling.**
  ACM Transactions on Graphics 37(4), SIGGRAPH 2018.
  [doi:10.1145/3197517.3201293](https://doi.org/10.1145/3197517.3201293)

  The MLS-MPM form of the internal forces, which reuses the affine velocity field
  for the grid forces and the deformation gradient update.
  Also the Compatible Particle-In-Cell (CPIC) method behind the collider handling:
  particles and grid nodes track which side of each collider they are on,
  transfers only happen between compatible ones,
  and particles that cross a collider surface are pushed back.

### Collision

- J. Andreas Bærentzen, Henrik Aanæs.
  **Signed Distance Computation Using the Angle Weighted Pseudonormal.**
  IEEE Transactions on Visualization and Computer Graphics 11(3), 2005.
  [doi:10.1109/TVCG.2005.49](https://doi.org/10.1109/TVCG.2005.49)

  Deciding on which side of a collider mesh a particle is, using angle-weighted vertex normals and edge normals.

### Elasticity

- Theodore Kim, David Eberle.
  **Dynamic Deformables: Implementation and Production Practicalities.**
  ACM SIGGRAPH 2020 Courses.
  [doi:10.1145/3388769.3407490](https://doi.org/10.1145/3388769.3407490)

  The invariant-based formulation of the neo-Hookean energy, its stress and Hessian.

- Breannan Smith, Fernando de Goes, Theodore Kim.
  **Stable Neo-Hookean Flesh Simulation.**
  ACM Transactions on Graphics 37(2), 2018.
  [doi:10.1145/3180491](https://doi.org/10.1145/3180491)

  The stable neo-Hookean energy and its Lamé reparameterization.

- Craig Schroeder.
  **Practical Course on Computing Derivatives in Code.**
  ACM SIGGRAPH 2019 Courses.
  [doi:10.1145/3305366.3328073](https://doi.org/10.1145/3305366.3328073)

  Derivatives of energies defined in terms of singular values.

### Sand

- Gergely Klár, Theodore Gast, Andre Pradhana, Chuyuan Fu, Craig Schroeder, Chenfanfu Jiang, Joseph Teran.
  **Drucker-Prager Elastoplasticity for Sand Animation.**
  ACM Transactions on Graphics 35(4), SIGGRAPH 2016.
  [doi:10.1145/2897824.2925906](https://doi.org/10.1145/2897824.2925906)

  The Drucker-Prager return mapping in Hencky strain behind the sand material.

### Fluids

- Andre Pradhana Tampubolon, Theodore Gast, Gergely Klár, Chuyuan Fu, Joseph Teran, Chenfanfu Jiang, Ken Museth.
  **Multi-Species Simulation of Porous Sand and Water Mixtures.**
  ACM Transactions on Graphics 36(4), SIGGRAPH 2017.
  [doi:10.1145/3072959.3073651](https://doi.org/10.1145/3072959.3073651)

- Markus Becker, Matthias Teschner.
  **Weakly Compressible SPH for Free Surface Flows.**
  Eurographics/ACM SIGGRAPH Symposium on Computer Animation 2007.
  [doi:10.2312/SCA/SCA07/209-218](https://doi.org/10.2312/SCA/SCA07/209-218)

  Both relate to the weakly compressible, Tait-like equation of state of the fluid material.

- L. D. Landau, E. M. Lifshitz.
  **Fluid Mechanics.**
  Course of Theoretical Physics, Volume 6, second edition, Pergamon Press, 1987.
  ISBN 978-0-08-033933-7

  The viscous stress with dynamic and bulk viscosity.

### Time Step Size

- Yunxin Sun, Tamar Shinar, Craig Schroeder.
  **Effective Time Step Restrictions for Explicit MPM Simulation.**
  Computer Graphics Forum 39(8), Symposium on Computer Animation 2020.
  [doi:10.1111/cgf.14101](https://doi.org/10.1111/cgf.14101)

  The sound speed CFL restriction and the velocity, deformation and single-particle fluid restrictions.

- Song Bai, Craig Schroeder.
  **Stability Analysis of Explicit MPM.**
  Computer Graphics Forum 41(8), Symposium on Computer Animation 2022.
  [doi:10.1111/cgf.14620](https://doi.org/10.1111/cgf.14620)

  The single-particle time step restriction, which the viscous limit extends to damping.

### GPU Algorithms

- Aleka McAdams, Andrew Selle, Rasmus Tamstorf, Joseph Teran, Eftychios Sifakis.
  **Computing the Singular Value Decomposition of 3x3 Matrices with Minimal Branching and Elementary Floating Point Operations.**
  Technical Report 1690, Computer Sciences Department, University of Wisconsin-Madison, 2011.
  [TR1690.pdf](https://research.cs.wisc.edu/techreports/2011/TR1690.pdf),
  [project page](https://pages.cs.wisc.edu/~sifakis/project_pages/svd.html)

  The algorithm behind the GPU 3x3 SVD, see wgebra and tbtSVD below.

- Takahiro Harada, Lee Howes.
  **Introduction to GPU Radix Sort.**
  Bonus chapter of *Heterogeneous Computing with OpenCL*, Morgan Kaufmann, 2011.
  [hgpu.org listing](https://hgpu.org/?p=5888)

  The GPU radix sort.

## Articles and Blog Posts

- David Farrell.
  **A Simple GPU Hash Table.**
  Blog post, 2020.
  [nosferalatu.com/SimpleGPUHashTable.html](https://nosferalatu.com/SimpleGPUHashTable.html)

  The lock-free, linearly probed GPU hash tables for grid nodes.

- Wikipedia.
  **Lamé parameters.**
  [en.wikipedia.org/wiki/Lamé_parameters](https://en.wikipedia.org/wiki/Lam%C3%A9_parameters)

  Conversion from Young's modulus and Poisson's ratio to the Lamé parameters.

- Gorgious.
  **Duplicating PointerProperty (PropertyGroup) and CollectionProperty.**
  Blender Artists forum answer, 2022.
  [blenderartists.org](https://blenderartists.org/t/duplicating-pointerproperty-propertygroup-and-collectionproperty/1419096/2)

  Copying the properties of a Blender property group.

## Code and Algorithms

- **wgebra**, part of wgmath by Sébastien Crozet.
  [github.com/wgmath/wgmath](https://github.com/wgmath/wgmath),
  licensed under MIT or Apache-2.0.

  The GPU 3x3 SVD and quaternion shaders are copied from wgebra, with formatting and imports adjusted.
  The license texts are in `rust/crates/gpu/src/shaders/external/wgebra/`.

- **tbtSVD** by wi-re.
  [github.com/wi-re/tbtSVD](https://github.com/wi-re/tbtSVD),
  licensed under MIT.

  wgebra's `svd3.wgsl` is a WGSL port of its `SVD.h`, an implementation of McAdams et al. 2011.

- **MurmurHash3** by Austin Appleby.
  [github.com/aappleby/smhasher](https://github.com/aappleby/smhasher),
  public domain.

  The 32-bit x86 variant hashes grid node ids on the GPU.
  The WGSL code is derived from the Rust [murmur3](https://github.com/stusmall/murmur3) crate by Stu Small,
  licensed under MIT or Apache-2.0, and tested against it.
