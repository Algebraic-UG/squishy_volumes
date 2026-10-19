// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

macro_rules! define_keys {
    ($(($variant:ident, $alembic_name:expr, $ty:ty)),* $(,)?) => {
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
        }

        $(
            pub struct $variant;
            impl Trait for $variant
            {
                const KEY: Key = Key::$variant;
                const ALEMBIC_NAME: &'static str = $alembic_name;
                type T = $ty;
            }
        )*
    };
}

pub mod attribute {
    pub trait Trait {
        const KEY: Key;
        const ALEMBIC_NAME: &str;
        type T: alembic_rust::AttributeElement;
    }

    pub trait Visitor {
        type Output;
        fn visit<T: Trait>(&mut self) -> Self::Output;
    }

    define_keys! {
        (Flags,                 "flags",                    [f32;2]),
        (Density,               "density",                  f32),
        (InitialVolumes,        "initial_volume",           f32),
        (ViscosityDynamic,      "viscosity_dynamic",        f32),
        (ViscosityBulk,         "viscosity_bulk",           f32),
        (YoungsModulus,         "youngs_modulus",           f32),
        (PoissonsRatio,         "poissons_ratio",           f32),
        (SandAlpha,             "sand_alpha",               f32),
        (BulkModulus,           "bulk_modulus",             f32),
        (Exponent,              "exponent",                 f32),
        (ElasticEnergy,         "elastic_energy",           f32),
        (ColliderBits,          "collider_bits",            [f32;2]),
        (Position,              alembic_rust::POSITIONS,    [f32;3]),
        (PositionGradientX,     "position_gradient_x",      [f32;3]),
        (PositionGradientY,     "position_gradient_y",      [f32;3]),
        (PositionGradientZ,     "position_gradient_z",      [f32;3]),
        (Velocity,              alembic_rust::VELOCITIES,   [f32;3]),
        (VelocityGradientX,     "velocity_gradient_x",      [f32;3]),
        (VelocityGradientY,     "velocity_gradient_y",      [f32;3]),
        (VelocityGradientZ,     "velocity_gradient_z",      [f32;3]),
        (InitialPosition,       "initial_position",         [f32;3]),
        (GoalPosition,          "goal_position",            [f32;3]),
    }
}
