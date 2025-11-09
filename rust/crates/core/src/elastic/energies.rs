// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use crate::{
    error_messages::INVERTED_PARTICLE,
    math::{Matrix9, SINGULAR_VALUE_SEPARATION, Vector9, safe_inverse::SafeInverse},
};
use anyhow::{Context, Result, ensure};
use nalgebra::{Matrix3, Normed, SVD, U3, Vector3, stack};
use squishy_volumes_api::T;

// Wikipedia: Lamé parameters (this is the "second")
pub fn mu(youngs_modulus: T, poissons_ratio: T) -> T {
    assert!(youngs_modulus >= 0.);
    assert!(poissons_ratio >= 0.);
    youngs_modulus / 2. / (1. + poissons_ratio)
}

// Wikipedia: Lamé parameters (this is the "first")
pub fn lambda(youngs_modulus: T, poissons_ratio: T) -> T {
    assert!(youngs_modulus >= 0.);
    assert!(poissons_ratio >= 0.);
    assert!(poissons_ratio < 0.5);
    youngs_modulus * poissons_ratio / (1. + poissons_ratio) / (1. - 2. * poissons_ratio)
}

// Stable Neo-Hookean Flesh Simulation 3.4 Lamé Reparameterization
pub fn mu_stable_neo_hookean(youngs_modulus: T, poissons_ratio: T) -> T {
    let mu = mu(youngs_modulus, poissons_ratio);
    4. / 3. * mu
}

// Stable Neo-Hookean Flesh Simulation 3.4 Lamé Reparameterization
pub fn lambda_stable_neo_hookean(youngs_modulus: T, poissons_ratio: T) -> T {
    let mu = mu(youngs_modulus, poissons_ratio);
    let lambda = lambda(youngs_modulus, poissons_ratio);
    lambda + 5. / 6. * mu
}

// Dynamic Deformables Implementation and Production Practicalities (B.7)
pub fn invariant_2(position_gradient: &Matrix3<T>) -> T {
    position_gradient.norm_squared()
}

pub fn invariant_2_by_svd(singular_values: &Vector3<T>) -> T {
    singular_values.norm_squared()
}

// Dynamic Deformables Implementation and Production Practicalities (B.20)
#[allow(clippy::toplevel_ref_arg)]
pub fn partial_invariant_2_by_position_gradient(position_gradient: &Matrix3<T>) -> Matrix3<T> {
    2. * position_gradient
}

pub fn partial_invariant_2_by_svd(singular_values: &Vector3<T>) -> Vector3<T> {
    2. * *singular_values
}

// Dynamic Deformables Implementation and Production Practicalities (B.8)
pub fn invariant_3(position_gradient: &Matrix3<T>) -> T {
    position_gradient.determinant()
}

pub fn invariant_3_by_svd(singular_values: &Vector3<T>) -> T {
    singular_values.product()
}

// Dynamic Deformables Implementation and Production Practicalities (B.23)
#[allow(clippy::toplevel_ref_arg)]
pub fn partial_invariant_3_by_position_gradient(position_gradient: &Matrix3<T>) -> Matrix3<T> {
    let c = |i| position_gradient.column(i);
    stack![
        c(1).cross(&c(2)), c(2).cross(&c(0)), c(0).cross(&c(1));
    ]
}

pub fn partial_invariant_3_by_svd(singular_values: &Vector3<T>) -> Vector3<T> {
    Vector3::new(
        singular_values.y * singular_values.z,
        singular_values.x * singular_values.z,
        singular_values.x * singular_values.y,
    )
}

pub fn double_partial_invariant_3_by_svd(singular_values: &Vector3<T>) -> Matrix3<T> {
    let x = singular_values.x;
    let y = singular_values.y;
    let z = singular_values.z;
    Matrix3::from_column_slice(&[
        0., z, y, //
        z, 0., x, //
        y, x, 0., //
    ])
}

// Dynamic Deformables Implementation and Production Practicalities (4.24)
#[allow(clippy::toplevel_ref_arg)]
pub fn double_partial_invariant_3_by_position_gradient(
    position_gradient: &Matrix3<T>,
) -> Matrix9<T> {
    let c = |i| position_gradient.column(i);
    stack![
        0, (-c(2)).cross_matrix(), c(1).cross_matrix();
        c(2).cross_matrix(), 0, (-c(0)).cross_matrix();
        (-c(1)).cross_matrix(), c(0).cross_matrix(), 0;
    ]
}

// Dynamic Deformables Implementation and Production Practicalities (6.9)
pub fn elastic_energy_stable_neo_hookean(mu: T, lambda: T, position_gradient: &Matrix3<T>) -> T {
    mu / 2. * (invariant_2(position_gradient) - 3.) - mu * (invariant_3(position_gradient) - 1.)
        + lambda / 2. * (invariant_3(position_gradient) - 1.).powi(2)
}

pub fn partial_elastic_energy_stable_neo_hookean_by_invariant_2(mu: T) -> T {
    mu / 2.
}

pub fn partial_elastic_energy_stable_neo_hookean_by_invariant_3(
    mu: T,
    lambda: T,
    invariant_3: T,
) -> T {
    lambda * (invariant_3 - 1.) - mu
}

pub fn first_piola_stress_stable_neo_hookean(
    mu: T,
    lambda: T,
    position_gradient: &Matrix3<T>,
) -> Matrix3<T> {
    partial_elastic_energy_stable_neo_hookean_by_invariant_2(mu)
        * partial_invariant_2_by_position_gradient(position_gradient)
        + partial_elastic_energy_stable_neo_hookean_by_invariant_3(
            mu,
            lambda,
            invariant_3(position_gradient),
        ) * partial_invariant_3_by_position_gradient(position_gradient)
}

pub fn first_piola_stress_stable_neo_hookean_svd(
    mu: T,
    lambda: T,
    u: &Matrix3<T>,
    s: &Vector3<T>,
    v_t: &Matrix3<T>,
) -> Matrix3<T> {
    u * Matrix3::from_diagonal(
        &(partial_elastic_energy_stable_neo_hookean_by_invariant_2(mu)
            * partial_invariant_2_by_svd(s)
            + partial_elastic_energy_stable_neo_hookean_by_invariant_3(
                mu,
                lambda,
                invariant_3_by_svd(s),
            ) * partial_invariant_3_by_svd(s)),
    ) * v_t
}

// The Material Point Method for Simulating Continuum Materials (46)
pub fn elastic_energy_neo_hookean_old(mu: T, lambda: T, position_gradient: &Matrix3<T>) -> T {
    mu / 2. * ((position_gradient.transpose() * position_gradient).trace() - 3.)
        - mu * position_gradient.determinant().ln()
        + lambda / 2. * position_gradient.determinant().ln().powi(2)
}

// The Material Point Method for Simulating Continuum Materials (48)
pub fn first_piola_stress_neo_hookean_old(
    mu: T,
    lambda: T,
    position_gradient: &Matrix3<T>,
) -> Result<Matrix3<T>> {
    let position_gradient_inv = position_gradient
        .safe_inverse()
        .context("position gradient isn't invertible")?;
    let position_gradient_inv_t = position_gradient_inv.transpose();
    Ok((position_gradient - position_gradient_inv_t) * mu
        + position_gradient_inv_t * lambda * position_gradient.determinant().ln())
}

// Dynamic Deformables Implementation and Production Practicalities 5.5.1
pub fn elastic_energy_neo_hookean_by_invariants(
    mu: T,
    lambda: T,
    invariant_2: T,
    invariant_3: T,
) -> T {
    assert!(invariant_3 > 0.);
    mu / 2. * (invariant_2 - 3.) - mu * invariant_3.ln() + lambda / 2. * invariant_3.ln().powi(2)
}

// Dynamic Deformables Implementation and Production Practicalities 5.5.1
pub fn elastic_energy_neo_hookean(mu: T, lambda: T, position_gradient: &Matrix3<T>) -> T {
    elastic_energy_neo_hookean_by_invariants(
        mu,
        lambda,
        invariant_2(position_gradient),
        invariant_3(position_gradient),
    )
}

pub fn try_elastic_energy_neo_hookean(
    mu: T,
    lambda: T,
    position_gradient: &Matrix3<T>,
) -> Result<T> {
    let invariant_3 = invariant_3(position_gradient);
    ensure!(invariant_3 > 0., INVERTED_PARTICLE);
    Ok(elastic_energy_neo_hookean_by_invariants(
        mu,
        lambda,
        invariant_2(position_gradient),
        invariant_3,
    ))
}

// Dynamic Deformables Implementation and Production Practicalities (5.51)
pub fn partial_elastic_energy_neo_hookean_by_invariant_2(mu: T) -> T {
    mu / 2.
}

// Dynamic Deformables Implementation and Production Practicalities (5.53)
pub fn partial_elastic_energy_neo_hookean_by_invariant_3(mu: T, lambda: T, invariant_3: T) -> T {
    (lambda * invariant_3.ln() - mu) / invariant_3
}

// Dynamic Deformables Implementation and Production Practicalities (5.53)
pub fn double_partial_elastic_energy_neo_hookean_by_invariant_3(
    mu: T,
    lambda: T,
    invariant_3: T,
) -> T {
    assert!(invariant_3 > 0.);
    (lambda * (1. - invariant_3.ln()) + mu) / invariant_3.powi(2)
}

// The Material Point Method for Simulating Continuum Materials (48)
pub fn first_piola_stress_neo_hookean(
    mu: T,
    lambda: T,
    position_gradient: &Matrix3<T>,
) -> Matrix3<T> {
    partial_elastic_energy_neo_hookean_by_invariant_2(mu)
        * partial_invariant_2_by_position_gradient(position_gradient)
        + partial_elastic_energy_neo_hookean_by_invariant_3(
            mu,
            lambda,
            invariant_3(position_gradient),
        ) * partial_invariant_3_by_position_gradient(position_gradient)
}

pub fn first_piola_stress_neo_hookean_svd_in_diagonal_space(
    mu: T,
    lambda: T,
    s: &Vector3<T>,
) -> Vector3<T> {
    partial_elastic_energy_neo_hookean_by_invariant_2(mu) * partial_invariant_2_by_svd(s)
        + partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3_by_svd(s))
            * partial_invariant_3_by_svd(s)
}

pub fn second_derivative_neo_hookean_svd_in_diagonal_space(
    mu: T,
    lambda: T,
    s: &Vector3<T>,
) -> Matrix3<T> {
    Matrix3::from_diagonal_element(partial_elastic_energy_neo_hookean_by_invariant_2(mu) * 2.)
        + double_partial_elastic_energy_neo_hookean_by_invariant_3(
            mu,
            lambda,
            invariant_3_by_svd(s),
        ) * partial_invariant_3_by_svd(s)
            * partial_invariant_3_by_svd(s).transpose()
        + partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3_by_svd(s))
            * double_partial_invariant_3_by_svd(s)
}

pub fn first_piola_stress_neo_hookean_svd(
    mu: T,
    lambda: T,
    u: &Matrix3<T>,
    s: &Vector3<T>,
    v_t: &Matrix3<T>,
) -> Matrix3<T> {
    u * Matrix3::from_diagonal(&first_piola_stress_neo_hookean_svd_in_diagonal_space(
        mu, lambda, s,
    )) * v_t
}

// Dynamic Deformables Implementation and Production Practicalities 5.5.1
pub fn hessian_neo_hookean(mu: T, lambda: T, position_gradient: &Matrix3<T>) -> Matrix9<T> {
    let g = Vector9::from_iterator(
        partial_invariant_3_by_position_gradient(position_gradient)
            .iter()
            .cloned(),
    );
    let invariant_3 = invariant_3(position_gradient);
    mu * Matrix9::identity()
        + double_partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3)
            * g
            * g.transpose()
        + partial_elastic_energy_neo_hookean_by_invariant_3(mu, lambda, invariant_3)
            * double_partial_invariant_3_by_position_gradient(position_gradient)
}

// Practical course on computing derivatives in code 5.1
pub fn hessian_neo_hookean_svd(
    mu: T,
    lambda: T,
    u: &Matrix3<T>,
    s: &Vector3<T>,
    v_t: &Matrix3<T>,
) -> Matrix9<T> {
    // to map from/to "diagonal space"
    let big_uv = v_t.transpose().kronecker(u);

    // first and second derivatives
    let psi_by_s = first_piola_stress_neo_hookean_svd_in_diagonal_space(mu, lambda, s);
    let double_psi_by_s = second_derivative_neo_hookean_svd_in_diagonal_space(mu, lambda, s);

    // we need to use L'Hôpital in this case
    let xy_close = (s.x - s.y).abs() < SINGULAR_VALUE_SEPARATION;
    let yz_close = (s.y - s.z).abs() < SINGULAR_VALUE_SEPARATION;
    let zx_close = (s.z - s.x).abs() < SINGULAR_VALUE_SEPARATION;

    // intermediates, should all be symmetrical and we don't need xx, yy, zz
    let a_xy = if xy_close {
        double_psi_by_s.m11 - double_psi_by_s.m21
    } else {
        (psi_by_s.x - psi_by_s.y) / (s.x - s.y)
    };
    let a_yz = if yz_close {
        double_psi_by_s.m22 - double_psi_by_s.m32
    } else {
        (psi_by_s.y - psi_by_s.z) / (s.y - s.z)
    };
    let a_zx = if zx_close {
        double_psi_by_s.m33 - double_psi_by_s.m13
    } else {
        (psi_by_s.z - psi_by_s.x) / (s.z - s.x)
    };
    let a_yx = a_xy;
    let a_zy = a_yz;
    let a_xz = a_zx;

    let b_xy = (psi_by_s.x + psi_by_s.y) / (s.x + s.y);
    let b_yz = (psi_by_s.y + psi_by_s.z) / (s.y + s.z);
    let b_zx = (psi_by_s.z + psi_by_s.x) / (s.z + s.x);
    let b_yx = b_xy;
    let b_zy = b_yz;
    let b_xz = b_zx;

    let t_xxxx = double_psi_by_s.m11;
    let t_xxxy = 0.;
    let t_xxxz = 0.;
    let t_xxyx = 0.;
    let t_xxyy = double_psi_by_s.m12;
    let t_xxyz = 0.;
    let t_xxzx = 0.;
    let t_xxzy = 0.;
    let t_xxzz = double_psi_by_s.m13;

    let t_xyxx = 0.;
    let t_xyxy = (a_xy + b_xy) / 2.;
    let t_xyxz = 0.;
    let t_xyyx = (a_xy - b_xy) / 2.;
    let t_xyyy = 0.;
    let t_xyyz = 0.;
    let t_xyzx = 0.;
    let t_xyzy = 0.;
    let t_xyzz = 0.;

    let t_xzxx = 0.;
    let t_xzxy = 0.;
    let t_xzxz = (a_xz + b_xz) / 2.;
    let t_xzyx = 0.;
    let t_xzyy = 0.;
    let t_xzyz = 0.;
    let t_xzzx = (a_xz - b_xz) / 2.;
    let t_xzzy = 0.;
    let t_xzzz = 0.;

    let t_yxxx = 0.;
    let t_yxxy = (a_yx - b_yx) / 2.;
    let t_yxxz = 0.;
    let t_yxyx = (a_yx + b_yx) / 2.;
    let t_yxyy = 0.;
    let t_yxyz = 0.;
    let t_yxzx = 0.;
    let t_yxzy = 0.;
    let t_yxzz = 0.;

    let t_yyxx = double_psi_by_s.m21;
    let t_yyxy = 0.;
    let t_yyxz = 0.;
    let t_yyyx = 0.;
    let t_yyyy = double_psi_by_s.m22;
    let t_yyyz = 0.;
    let t_yyzx = 0.;
    let t_yyzy = 0.;
    let t_yyzz = double_psi_by_s.m23;

    let t_yzxx = 0.;
    let t_yzxy = 0.;
    let t_yzxz = 0.;
    let t_yzyx = 0.;
    let t_yzyy = 0.;
    let t_yzyz = (a_yz + b_yz) / 2.;
    let t_yzzx = 0.;
    let t_yzzy = (a_yz - b_yz) / 2.;
    let t_yzzz = 0.;

    let t_zxxx = 0.;
    let t_zxxy = 0.;
    let t_zxxz = (a_zx - b_zx) / 2.;
    let t_zxyx = 0.;
    let t_zxyy = 0.;
    let t_zxyz = 0.;
    let t_zxzx = (a_zx + b_zx) / 2.;
    let t_zxzy = 0.;
    let t_zxzz = 0.;

    let t_zyxx = 0.;
    let t_zyxy = 0.;
    let t_zyxz = 0.;
    let t_zyyx = 0.;
    let t_zyyy = 0.;
    let t_zyyz = (a_zy - b_zy) / 2.;
    let t_zyzx = 0.;
    let t_zyzy = (a_zy + b_zy) / 2.;
    let t_zyzz = 0.;

    let t_zzxx = double_psi_by_s.m31;
    let t_zzxy = 0.;
    let t_zzxz = 0.;
    let t_zzyx = 0.;
    let t_zzyy = double_psi_by_s.m32;
    let t_zzyz = 0.;
    let t_zzzx = 0.;
    let t_zzzy = 0.;
    let t_zzzz = double_psi_by_s.m33;

    let singular_space_hessian = Matrix9::from_column_slice(&[
        t_xxxx, t_xxxy, t_xxxz, t_xxyx, t_xxyy, t_xxyz, t_xxzx, t_xxzy, t_xxzz, //
        t_xyxx, t_xyxy, t_xyxz, t_xyyx, t_xyyy, t_xyyz, t_xyzx, t_xyzy, t_xyzz, //
        t_xzxx, t_xzxy, t_xzxz, t_xzyx, t_xzyy, t_xzyz, t_xzzx, t_xzzy, t_xzzz, //
        t_yxxx, t_yxxy, t_yxxz, t_yxyx, t_yxyy, t_yxyz, t_yxzx, t_yxzy, t_yxzz, //
        t_yyxx, t_yyxy, t_yyxz, t_yyyx, t_yyyy, t_yyyz, t_yyzx, t_yyzy, t_yyzz, //
        t_yzxx, t_yzxy, t_yzxz, t_yzyx, t_yzyy, t_yzyz, t_yzzx, t_yzzy, t_yzzz, //
        t_zxxx, t_zxxy, t_zxxz, t_zxyx, t_zxyy, t_zxyz, t_zxzx, t_zxzy, t_zxzz, //
        t_zyxx, t_zyxy, t_zyxz, t_zyyx, t_zyyy, t_zyyz, t_zyzx, t_zyzy, t_zyzz, //
        t_zzxx, t_zzxy, t_zzxz, t_zzyx, t_zzyy, t_zzyz, t_zzzx, t_zzzy, t_zzzz, //
    ]);

    big_uv * singular_space_hessian * big_uv.transpose()
}

// Something like
// Multi-species simulation of porous sand and water mixtures (11)
// Weakly compressible SPH for free surface flows (7)
pub fn elastic_energy_inviscid_by_invariant(bulk_modulus: T, exponent: i32, invariant_3: T) -> T {
    assert!(bulk_modulus >= 0.);
    assert!(exponent > 1);
    // https://github.com/Algebraic-UG/squishy_volumes/issues/125
    let at_rest = bulk_modulus * (1. - 1. / (1. - exponent as T));
    bulk_modulus * (invariant_3 - invariant_3.powi(1 - exponent) / (1. - exponent as T)) - at_rest
}

pub fn partial_elastic_energy_inviscid_by_invariant_3(
    bulk_modulus: T,
    exponent: i32,
    invariant_3: T,
) -> T {
    bulk_modulus * (1. - 1. / invariant_3.powi(exponent))
}

pub fn double_partial_elastic_energy_inviscid_by_invariant_3(
    bulk_modulus: T,
    exponent: i32,
    invariant_3: T,
) -> T {
    exponent as T * bulk_modulus / invariant_3.powi(exponent + 1)
}

pub fn elastic_energy_inviscid(
    bulk_modulus: T,
    exponent: i32,
    position_gradient: &Matrix3<T>,
) -> T {
    elastic_energy_inviscid_by_invariant(bulk_modulus, exponent, invariant_3(position_gradient))
}

pub fn first_piola_stress_inviscid(
    bulk_modulus: T,
    exponent: i32,
    position_gradient: &Matrix3<T>,
) -> Matrix3<T> {
    partial_elastic_energy_inviscid_by_invariant_3(
        bulk_modulus,
        exponent,
        invariant_3(position_gradient),
    ) * partial_invariant_3_by_position_gradient(position_gradient)
}

pub fn first_piola_stress_inviscid_svd(
    bulk_modulus: T,
    exponent: i32,
    u: &Matrix3<T>,
    s: &Vector3<T>,
    v_t: &Matrix3<T>,
) -> Matrix3<T> {
    u * Matrix3::from_diagonal(&first_piola_stress_inviscid_svd_in_diagonal_space(
        bulk_modulus,
        exponent,
        s,
    )) * v_t
}

// Dynamic Deformables Implementation and Production Practicalities 5.50
pub fn hessian_inviscid(
    bulk_modulus: T,
    exponent: i32,
    position_gradient: &Matrix3<T>,
) -> Matrix9<T> {
    let g = Vector9::from_iterator(
        partial_invariant_3_by_position_gradient(position_gradient)
            .iter()
            .cloned(),
    );
    let invariant_3 = invariant_3(position_gradient);
    double_partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, invariant_3)
        * g
        * g.transpose()
        + partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, invariant_3)
            * double_partial_invariant_3_by_position_gradient(position_gradient)
}

pub fn first_piola_stress_inviscid_svd_in_diagonal_space(
    bulk_modulus: T,
    exponent: i32,
    s: &Vector3<T>,
) -> Vector3<T> {
    partial_elastic_energy_inviscid_by_invariant_3(bulk_modulus, exponent, invariant_3_by_svd(s))
        * partial_invariant_3_by_svd(s)
}

pub fn second_derivative_inviscid_svd_in_diagonal_space(
    bulk_modulus: T,
    exponent: i32,
    s: &Vector3<T>,
) -> Matrix3<T> {
    double_partial_elastic_energy_inviscid_by_invariant_3(
        bulk_modulus,
        exponent,
        invariant_3_by_svd(s),
    ) * partial_invariant_3_by_svd(s)
        * partial_invariant_3_by_svd(s).transpose()
        + partial_elastic_energy_inviscid_by_invariant_3(
            bulk_modulus,
            exponent,
            invariant_3_by_svd(s),
        ) * double_partial_invariant_3_by_svd(s)
}

pub fn rate_of_strain(velocity_gradient: &Matrix3<T>) -> Matrix3<T> {
    0.5 * (velocity_gradient + velocity_gradient.transpose())
}

pub fn velocity_divergence(velocity_gradient: &Matrix3<T>) -> T {
    velocity_gradient.trace()
}

// Fluid Mechanics, Second Edition, L. D. Landau and E. M. Lifshitz (15.3)
pub fn cauchy_stress_general_viscosity(
    dynamic_viscosity: T,
    bulk_viscosity: T,
    velocity_gradient: &Matrix3<T>,
) -> Matrix3<T> {
    2. * dynamic_viscosity * rate_of_strain(velocity_gradient)
        + bulk_viscosity * Matrix3::from_diagonal_element(velocity_divergence(velocity_gradient))
}
