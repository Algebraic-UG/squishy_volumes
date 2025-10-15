// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result, ensure};
use nalgebra::{Matrix3, Normed, SVD, U3, Vector3, stack};
use squishy_volumes_api::T;

use crate::{
    math::{Matrix9, Vector9, safe_inverse::SafeInverse},
    simulation::error_messages::INVERTED_PARTICLE,
};

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

// Something like
// Multi-species simulation of porous sand and water mixtures (11)
// Weakly compressible SPH for free surface flows (7)
pub fn elastic_energy_inviscid_by_invariant(bulk_modulus: T, exponent: i32, invariant_3: T) -> T {
    assert!(bulk_modulus >= 0.);
    assert!(exponent > 1);
    bulk_modulus * (invariant_3 - invariant_3.powi(1 - exponent) / (1. - exponent as T))
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
