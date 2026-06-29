// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

import super::util::{Indirect, get_global_index, ParticleParameters, IS_SOLID, USE_SAND_ALPHA};
import super::svd::{Svd, svd, recompose};

@group(0) @binding(0)
var<storage, read_write> particle_parameters: array<ParticleParameters>;

@group(0) @binding(1)
var<storage, read_write> particle_position_gradients: array<mat3x3f>;

override WORKGROUP_SIZE: u32;

@compute @workgroup_size(WORKGROUP_SIZE)
fn main(
    @builtin(global_invocation_id) global_invocation_id: vec3<u32>,
    @builtin(num_workgroups) num_workgroups: vec3<u32>,
) {
    let global_index = get_global_index(WORKGROUP_SIZE, num_workgroups, global_invocation_id);
    if global_index >= arrayLength(&particle_parameters) {
        return;
    }

    let parameters = particle_parameters[global_index];

    if (parameters.flags & IS_SOLID) == 0 {
        return;
    }
    if (parameters.flags & USE_SAND_ALPHA) == 0 {
        return;
    }

    let mu = parameters.a;
    let lambda = parameters.b;
    let sand_alpha = parameters.e;

    particle_position_gradients[global_index] = sand_plasticity(
        mu,
        lambda,
        sand_alpha,
        particle_position_gradients[global_index],
    );
}

fn sand_plasticity(
    mu: f32,
    lambda: f32,
    sand_alpha: f32,
    position_gradient: mat3x3f
) -> mat3x3f {
    var svd = svd(position_gradient);
    let e = log(svd.S);
    let e_tr = e.x + e.y + e.z;
    let e_hat = e - vec3f(e_tr / 3.);
    let e_hat_norm = length(e_hat);
    if mu > 0. && e_tr < 0. && e_hat_norm > 0. {
        if e_hat_norm != 0. {
            let delta_gamma = e_hat_norm + (3. * lambda + 2. * mu) / 2. / mu * e_tr * sand_alpha;
            if delta_gamma > 0. {
                let big_h = e - delta_gamma / e_hat_norm * e_hat;
                svd.S = exp(big_h);
                return recompose(svd);
            }
        }
    } else {
        return svd.U * svd.Vt;
    }
    return position_gradient;
}
