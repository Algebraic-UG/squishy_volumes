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

macro_rules! define_keys {
    ($((
        $variant:ident,
        $alembic_name:expr,
        $ty:ty,
        $default:expr,
        $scale:expr,
    )),* $(,)?) => {
        #[derive(
            Clone,
            Copy,
            Debug,
            serde::Serialize,
            serde::Deserialize,
            PartialEq,
            Eq,
            PartialOrd,
            Ord,
            strum_macros::EnumIter,
        )]
        #[repr(u8)]
        pub enum Key {
            $($variant),*
        }

        impl Key {
            pub fn alembic_name(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => $alembic_name,
                    )*
                }
            }
            pub fn alembic_type(&self) -> alembic_rust::AttributeType {
                match self {
                    $(
                        Self::$variant => <$ty as alembic_rust::AttributeElement>::TYPE,
                    )*
                }
            }
            pub fn visit<V: Visitor>(&self, visitor: &mut V) -> V::Output {
                match self {
                    $(
                        Self::$variant => visitor.visit::<$variant>(),
                    )*
                }
            }
            pub fn scale(&self) -> bool {
                match self {
                    $(
                        Self::$variant => $scale,
                    )*
                }
            }

        }

        $(
            pub struct $variant;
            impl Trait for $variant
            {
                const KEY: Key = Key::$variant;
                const ALEMBIC_NAME: &'static str = $alembic_name;
                type T = $ty;
                const DEFAULT: Self::T = $default;
                const SCALE: bool = $scale;
            }
        )*
    };
}

pub mod attribute {
    pub trait Trait {
        const KEY: Key;
        const ALEMBIC_NAME: &str;
        type T: alembic_rust::AttributeElement;
        const DEFAULT: Self::T;
        const SCALE: bool;
    }

    pub trait Visitor {
        type Output;
        fn visit<T: Trait>(&mut self) -> Self::Output;
    }

    define_keys! {
        (Flags,                 "flags",                    [f32;2],    [0.;2],        false, ),
        (Density,               "density",                  f32,        1000.,         false, ),
        (InitialSize,           "initial_size",             f32,        0.25,          true,  ),
        (ViscosityDynamic,      "viscosity_dynamic",        f32,        3.,            false, ),
        (ViscosityBulk,         "viscosity_bulk",           f32,        3.,            false, ),
        (YoungsModulus,         "youngs_modulus",           f32,        10000.,        false, ),
        (PoissonsRatio,         "poissons_ratio",           f32,        0.3,           false, ),
        (SandAlpha,             "sand_alpha",               f32,        0.3,           false, ),
        (BulkModulus,           "bulk_modulus",             f32,        10000.,        false, ),
        (Exponent,              "exponent",                 f32,        2.,            false, ),
        (ElasticEnergy,         "elastic_energy",           f32,        0.,            false, ),
        (ColliderBits,          "collider_bits",            [f32;2],    [0.;2],        false, ),
        (Position,              alembic_rust::POSITIONS,    [f32;3],    [0.;3],        true,  ),
        (PositionGradientX,     "position_gradient_x",      [f32;3],    [1.,0.,0.],    false, ),
        (PositionGradientY,     "position_gradient_y",      [f32;3],    [0.,1.,0.],    false, ),
        (PositionGradientZ,     "position_gradient_z",      [f32;3],    [0.,0.,1.],    false, ),
        (Velocity,              "velocity",                 [f32;3],    [0.;3],        true,  ),
        (VelocityGradientX,     "velocity_gradient_x",      [f32;3],    [0.;3],        false, ),
        (VelocityGradientY,     "velocity_gradient_y",      [f32;3],    [0.;3],        false, ),
        (VelocityGradientZ,     "velocity_gradient_z",      [f32;3],    [0.;3],        false, ),
        (InitialPosition,       "initial_position",         [f32;3],    [0.;3],        true,  ),
        (GoalPosition,          "goal_position",            [f32;3],    [0.;3],        true,  ),
    }
}
