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

#[derive(Debug, Clone)]
pub enum Host {
    Solid(Solid),
    Fluid(Fluid),
}

#[derive(Debug, Clone)]
pub struct Viscosity {
    pub dynamic: f32,
    pub bulk: f32,
}

#[derive(Debug, Clone)]
pub struct Solid {
    pub mu: f32,
    pub lambda: f32,
    pub viscosity: Option<Viscosity>,
    pub sand_alpha: Option<f32>,
}

#[derive(Debug, Clone)]
pub struct Fluid {
    pub exponent: i32,
    pub bulk_modulus: f32,
    pub viscosity: Option<Viscosity>,
}

impl From<Host> for Device {
    fn from(value: Host) -> Self {
        let mut res = Self::default();
        match value {
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

impl From<Device> for Host {
    fn from(
        Device {
            flags,
            a,
            b,
            c,
            d,
            e,
        }: Device,
    ) -> Self {
        if flags.contains(Flags::IS_SOLID) {
            return Self::Solid(Solid {
                mu: a,
                lambda: b,
                viscosity: flags.contains(Flags::USE_VISCOSITY).then_some(Viscosity {
                    dynamic: c,
                    bulk: d,
                }),
                sand_alpha: flags.contains(Flags::USE_SAND_ALPHA).then_some(e),
            });
        }
        if flags.contains(Flags::IS_FLUID) {
            return Self::Fluid(Fluid {
                exponent: a as i32,
                bulk_modulus: b,
                viscosity: flags.contains(Flags::USE_VISCOSITY).then_some(Viscosity {
                    dynamic: c,
                    bulk: d,
                }),
            });
        }
        unreachable!()
    }
}

impl AllowedInBinding for Device {
    const ALIGNMENT: NonZeroU64 = u32::ALIGNMENT;
}
