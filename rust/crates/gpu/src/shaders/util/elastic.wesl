// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This file is part of the Squishy Volumes extension.
// Copyright (C) 2025-2026  Algebraic UG (haftungsbeschränkt)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

import super::matrix::{from_diagonal_element, outer};

fn invariant_2(position_gradient: mat3x3f) -> f32 {
    return dot(position_gradient[0], position_gradient[0]) +
        dot(position_gradient[1], position_gradient[1]) +
        dot(position_gradient[2], position_gradient[2]);
}

fn invariant_3(position_gradient: mat3x3f) -> f32 {
    return determinant(position_gradient);
}

fn elastic_energy_neo_hookean_by_invariants(
    mu: f32,
    lambda: f32,
    invariant_2: f32,
    invariant_3: f32,
) -> f32 {
    return mu / 2. * (invariant_2 - 3.) - mu * log(invariant_3) + lambda / 2. * log(invariant_3) * log(invariant_3);
}

fn elastic_energy_neo_hookean(
    mu: f32,
    lambda: f32,
    position_gradient: mat3x3f,
) -> f32 {
    return elastic_energy_neo_hookean_by_invariants(
        mu, lambda, invariant_2(position_gradient), invariant_3(position_gradient)
    );
}

fn elastic_energy_inviscid_by_invariant(bulk_modulus: f32, exponent: i32, invariant_3: f32) -> f32 {
    let at_rest = bulk_modulus * (1. - 1. / (1. - f32(exponent)));
    return bulk_modulus * (invariant_3 - pow(invariant_3, 1. - f32(exponent)) / (1. - f32(exponent))) - at_rest;
}

fn elastic_energy_inviscid(
    bulk_modulus: f32,
    exponent: i32,
    position_gradient: mat3x3f,
) -> f32 {
    return elastic_energy_inviscid_by_invariant(bulk_modulus, exponent, invariant_3(position_gradient));
}

fn partial_elastic_energy_neo_hookean_by_invariant_2(mu: f32) -> f32 {
    return mu / 2.;
}

fn partial_invariant_2_by_position_gradient(position_gradient: mat3x3f) -> mat3x3f {
    return 2. * position_gradient;
}

fn partial_elastic_energy_neo_hookean_by_invariant_3(mu: f32, lambda: f32, invariant_3: f32) -> f32 {
    return (lambda * log(invariant_3) - mu) / invariant_3;
}

fn partial_invariant_3_by_position_gradient(position_gradient: mat3x3f) -> mat3x3f {
    return mat3x3f(
        cross(position_gradient[1], position_gradient[2]),
        cross(position_gradient[2], position_gradient[0]),
        cross(position_gradient[0], position_gradient[1]),
    );
}

fn first_piola_stress_neo_hookean(
    mu: f32,
    lambda: f32,
    position_gradient: mat3x3f,
) -> mat3x3f {
    return partial_elastic_energy_neo_hookean_by_invariant_2(mu)
    * partial_invariant_2_by_position_gradient(position_gradient)
    + partial_elastic_energy_neo_hookean_by_invariant_3(
        mu,
        lambda,
        invariant_3(position_gradient),
    ) * partial_invariant_3_by_position_gradient(position_gradient);
}

fn partial_elastic_energy_inviscid_by_invariant_3(
    bulk_modulus: f32,
    exponent: i32,
    invariant_3: f32,
) -> f32 {
    return bulk_modulus * (1. - 1. / pow(invariant_3, f32(exponent)));
}

fn first_piola_stress_inviscid(
    bulk_modulus: f32,
    exponent: i32,
    position_gradient: mat3x3f,
) -> mat3x3f {
    return partial_elastic_energy_inviscid_by_invariant_3(
        bulk_modulus,
        exponent,
        invariant_3(position_gradient),
    ) * partial_invariant_3_by_position_gradient(position_gradient);
}

fn partial_invariant_2_by_svd(singular_values: vec3f) -> vec3f {
    return 2. * singular_values;
}

fn invariant_3_by_svd(singular_values: vec3f) -> f32 {
    return singular_values.x * singular_values.y * singular_values.z;
}

fn partial_invariant_3_by_svd(singular_values: vec3f) -> vec3f {
    return vec3f(
        singular_values.y * singular_values.z,
        singular_values.x * singular_values.z,
        singular_values.x * singular_values.y,
    );
}

fn double_partial_invariant_3_by_svd(singular_values: vec3f) -> mat3x3f {
    let x = singular_values.x;
    let y = singular_values.y;
    let z = singular_values.z;
    return mat3x3f(
        0., z, y,
        z, 0., x,
        y, x, 0.,
    );
}

fn first_piola_stress_neo_hookean_svd_in_diagonal_space(
    mu: f32,
    lambda: f32,
    s: vec3f,
) -> vec3f {
    return partial_elastic_energy_neo_hookean_by_invariant_2(mu) * partial_invariant_2_by_svd(s)
    + partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3_by_svd(s))
    * partial_invariant_3_by_svd(s);
}

fn double_partial_elastic_energy_neo_hookean_by_invariant_3(
    mu: f32,
    lambda: f32,
    invariant_3: f32,
) -> f32 {
    return (lambda * (1. - log(invariant_3)) + mu) / pow(invariant_3, 2);
}

fn second_derivative_neo_hookean_svd_in_diagonal_space(
    mu: f32,
    lambda: f32,
    s: vec3f,
) -> mat3x3f {
    return from_diagonal_element(partial_elastic_energy_neo_hookean_by_invariant_2(mu) * 2.)
    + outer(double_partial_elastic_energy_neo_hookean_by_invariant_3(
        mu,
        lambda,
        invariant_3_by_svd(s),
    ) * partial_invariant_3_by_svd(s),
        partial_invariant_3_by_svd(s))
    + partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3_by_svd(s))
    * double_partial_invariant_3_by_svd(s);
}

fn first_piola_stress_inviscid_svd_in_diagonal_space(
    bulk_modulus: f32,
    exponent: i32,
    s: vec3f,
) -> vec3f {
    return partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, invariant_3_by_svd(s))
    * partial_invariant_3_by_svd(s);
}

fn double_partial_elastic_energy_inviscid_by_invariant_3(
    bulk_modulus: f32,
    exponent: i32,
    invariant_3: f32,
) -> f32 {
    return f32(exponent) * bulk_modulus / pow(invariant_3, f32(exponent) + 1.);
}

fn second_derivative_inviscid_svd_in_diagonal_space(
    bulk_modulus: f32,
    exponent: i32,
    s: vec3f,
) -> mat3x3f {
    return outer(double_partial_elastic_energy_inviscid_by_invariant_3(
        bulk_modulus,
        exponent,
        invariant_3_by_svd(s),
    ) * partial_invariant_3_by_svd(s),
        partial_invariant_3_by_svd(s))
    + partial_elastic_energy_inviscid_by_invariant_3(
        bulk_modulus,
        exponent,
        invariant_3_by_svd(s),
    ) * double_partial_invariant_3_by_svd(s);
}
