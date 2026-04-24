// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::num::NonZeroU64;

use bitflags::bitflags;

use crate::AllowedInBinding;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
struct Flags(u32);
bitflags! {
    impl Flags: u32{
        const IS_SOLID = 1;
        const IS_FLUID = 1 << 1;
        const USE_VISCOSITY = 1 << 2;
        const USE_SAND_ALPHA = 1 << 3;
    }
}

// TODO: this is too ugly
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Zeroable, bytemuck::Pod, Debug, PartialEq, Default)]
pub struct Device {
    flags: Flags,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
}

pub enum Host {
    Solid(Solid),
    Fluid(Fluid),
}

pub struct Viscosity {
    pub dynamic: f32,
    pub bulk: f32,
}

pub struct Solid {
    pub mu: f32,
    pub lambda: f32,
    pub viscosity: Option<Viscosity>,
    pub sand_alpha: Option<f32>,
}

pub struct Fluid {
    pub exponent: i32,
    pub bulk_modulus: f32,
    pub viscosity: Option<Viscosity>,
}

impl Device {
    pub fn new(host: Host) -> Self {
        let mut res = Self::default();
        match host {
            Host::Solid(Solid {
                mu,
                lambda,
                viscosity,
                sand_alpha,
            }) => {
                res.flags |= Flags::IS_SOLID;
                res.a = mu;
                res.b = lambda;
                if let Some(viscosity) = viscosity {
                    res.flags |= Flags::USE_VISCOSITY;
                    res.c = viscosity.dynamic;
                    res.d = viscosity.bulk;
                }
                if let Some(sand_alpha) = sand_alpha {
                    res.flags |= Flags::USE_SAND_ALPHA;
                    res.e = sand_alpha;
                }
            }
            Host::Fluid(Fluid {
                exponent,
                bulk_modulus,
                viscosity,
            }) => {
                res.flags |= Flags::IS_FLUID;
                res.a = exponent as f32;
                res.b = bulk_modulus;
                if let Some(viscosity) = viscosity {
                    res.flags |= Flags::USE_VISCOSITY;
                    res.c = viscosity.dynamic;
                    res.d = viscosity.bulk;
                }
            }
        }
        res
    }
}

impl AllowedInBinding for Device {
    const ALIGNMENT: NonZeroU64 = u32::ALIGNMENT;
}
