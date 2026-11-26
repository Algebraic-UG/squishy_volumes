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

use std::{borrow::Cow, num::NonZero, path::Path};

use squishy_volumes_api::InputBulk;
use squishy_volumes_cpu::{CpuRunParameters, CpuState};
use squishy_volumes_file_frame::{IoState, two_f32_to_u32, u32_to_two_f32};
use squishy_volumes_file_input::{
    BulkAttribute, FrameBulk, FrameBulkMeta, FrameBulkParticles, InputHeader, InputObject,
    InputObjectParticles, InputReader, write_test_input_frames,
};
use squishy_volumes_gpu::{GpuContext, GpuRunParameters, GpuState};
use squishy_volumes_util::{AnimatedGlobals, ParticleFlags, attribute};
use squishy_volumes_xpu::Harness;
use tempfile::TempDir;

use crate::initialization::initialize_io_state;

const SIMULATION_SCALE: f32 = 2.;

const FLAG_MASKS_AND_VALUES: [[ParticleFlags; 2]; 3] = [
    [ParticleFlags::HAS_GOAL, ParticleFlags::HAS_GOAL],
    [
        ParticleFlags::USE_VISCOSITY.union(ParticleFlags::IS_ACTIVE),
        ParticleFlags::USE_VISCOSITY.union(ParticleFlags::IS_FLUID),
    ],
    [
        ParticleFlags::HAS_GOAL.union(ParticleFlags::USE_VISCOSITY),
        ParticleFlags::HAS_GOAL,
    ],
];
const OVERWRITTEN_FLAGS: [ParticleFlags; 3] = [
    ParticleFlags::IS_SOLID
        .union(ParticleFlags::IS_ACTIVE)
        .union(ParticleFlags::HAS_GOAL),
    ParticleFlags::IS_SOLID.union(ParticleFlags::USE_VISCOSITY),
    ParticleFlags::IS_SOLID
        .union(ParticleFlags::FAILED)
        .union(ParticleFlags::HAS_GOAL),
];
const VELOCITIES: [[f32; 3]; 3] = [[1., 2., 3.], [-1., 0., 0.5], [0.25, -0.5, 0.]];
const GOAL_POSITIONS: [[f32; 3]; 3] = [[4., 1., 0.], [1., 1., 1.], [-2., 0., 3.]];
const INITIAL_POSITIONS: [[f32; 3]; 3] = [[4., 0., 0.], [0., 4., 0.], [0., 0., 4.]];
const DENSITIES: [f32; 3] = [500., 600., 700.];
const SIZES: [f32; 3] = [0.2, 0.3, 0.4];
const YOUNGS_MODULI: [f32; 3] = [2e4, 3e4, 4e4];
const EXPONENTS: [i32; 3] = [3, 4, 5];
const POISSONS_RATIOS: [f32; 3] = [0.2, 0.25, 0.35];
const VISCOSITIES_DYNAMIC: [f32; 3] = [1., 2., 3.];
const VISCOSITIES_BULK: [f32; 3] = [4., 5., 6.];
const BULK_MODULI: [f32; 3] = [5e4, 6e4, 7e4];
const SAND_ALPHAS: [f32; 3] = [0.1, 0.2, 0.3];
const COLLIDER_BITS: [u32; 3] = [1, 2, 0b101];
const POSITIONS: [[f32; 3]; 3] = [[1.5, 1., 0.], [1., 1., 0.], [0.5, 1., 0.]];
const GRADIENT_COLUMNS: [[f32; 3]; 3] = [[1.1, 0., 0.], [0.1, 0.9, 0.], [0., 0., 1.]];

fn globals() -> AnimatedGlobals {
    AnimatedGlobals {
        gravity_x: 0.,
        gravity_y: 0.,
        gravity_z: -9.81,
        goal_stiffness: 100.,
        goal_damping: 0.,
        damping: 0.,
    }
}

fn transform(position: [f32; 3], columns: [[f32; 3]; 3]) -> [[f32; 4]; 4] {
    let [x, y, z] = columns.map(|[a, b, c]| [a, b, c, 0.]);
    let [px, py, pz] = position;
    [x, y, z, [px, py, pz, 1.]]
}

fn bulk(
    object: &str,
    captured_attribute: BulkAttribute,
    data: InputBulk<'static>,
) -> FrameBulk<'static> {
    FrameBulk {
        meta: FrameBulkMeta {
            object_name: object.to_string(),
            captured_attribute,
        },
        data,
    }
}

fn particle_bulk(
    object: &str,
    attribute: FrameBulkParticles,
    data: InputBulk<'static>,
) -> FrameBulk<'static> {
    bulk(object, BulkAttribute::Particles(attribute), data)
}

fn floats<const N: usize, T: bytemuck::Pod>(values: [T; N]) -> InputBulk<'static> {
    InputBulk::Floats(Cow::Owned(bytemuck::cast_slice(&values).to_vec()))
}

fn ints<const N: usize, T: bytemuck::Pod>(values: [T; N]) -> InputBulk<'static> {
    InputBulk::Ints(Cow::Owned(bytemuck::cast_slice(&values).to_vec()))
}

fn first_frame(object: &str, positions: &[[f32; 3]]) -> Vec<FrameBulk<'static>> {
    let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
    let transforms: Vec<[[f32; 4]; 4]> = positions
        .iter()
        .map(|position| transform(*position, identity))
        .collect();
    vec![
        particle_bulk(
            object,
            FrameBulkParticles::Flags,
            InputBulk::Ints(
                [
                    ParticleFlags::IS_SOLID | ParticleFlags::IS_FLUID,
                    ParticleFlags::IS_SOLID | ParticleFlags::IS_ACTIVE,
                ]
                .map(|flags| flags.bits() as i32)
                .repeat(positions.len())
                .into(),
            ),
        ),
        particle_bulk(
            object,
            FrameBulkParticles::Transform,
            InputBulk::Floats(Cow::Owned(bytemuck::cast_slice(&transforms).to_vec())),
        ),
    ]
}

fn second_frame() -> Vec<FrameBulk<'static>> {
    use FrameBulkParticles as P;
    vec![
        particle_bulk(
            "b",
            P::Flags,
            ints(FLAG_MASKS_AND_VALUES.map(|flags| flags.map(|flags| flags.bits()))),
        ),
        particle_bulk("b", P::Velocity, floats(VELOCITIES)),
        particle_bulk("b", P::GoalPosition, floats(GOAL_POSITIONS)),
        particle_bulk("b", P::InitialPosition, floats(INITIAL_POSITIONS)),
        particle_bulk("b", P::Density, floats(DENSITIES)),
        particle_bulk("b", P::Size, floats(SIZES)),
        particle_bulk("b", P::YoungsModulus, floats(YOUNGS_MODULI)),
        particle_bulk("b", P::Exponent, ints(EXPONENTS)),
        particle_bulk("b", P::PoissonsRatio, floats(POISSONS_RATIOS)),
        particle_bulk("b", P::ViscosityDynamic, floats(VISCOSITIES_DYNAMIC)),
        particle_bulk("b", P::ViscosityBulk, floats(VISCOSITIES_BULK)),
        particle_bulk("b", P::BulkModulus, floats(BULK_MODULI)),
        particle_bulk("b", P::SandAlpha, floats(SAND_ALPHAS)),
        particle_bulk("b", P::ColliderBits, ints(COLLIDER_BITS)),
        particle_bulk(
            "b",
            P::Transform,
            floats(POSITIONS.map(|position| transform(position, GRADIENT_COLUMNS))),
        ),
    ]
}

fn record(path: &Path) -> InputReader {
    let particles = |num_particles| {
        InputObject::Particles(InputObjectParticles {
            num_particles,
            alembic_attributes: Default::default(),
        })
    };
    let mut header = InputHeader::test_input([
        ("a".to_string(), particles(2)),
        ("b".to_string(), particles(3)),
    ]);
    header.consts.simulation_scale = SIMULATION_SCALE;

    let mut frame_0 = first_frame("a", &[[3., 0., 0.], [2.5, 0., 0.]]);
    frame_0.extend(first_frame(
        "b",
        &[[2., 0., 0.], [1.5, 0., 0.], [1., 0., 0.]],
    ));
    write_test_input_frames(
        path,
        header,
        [(globals(), frame_0), (globals(), second_frame())],
    )
}

fn harness() -> Harness {
    Harness::new("test".to_string(), NonZero::new(1).unwrap())
}

fn initial_state(input_reader: &mut InputReader) -> IoState {
    let mut io_state = initialize_io_state(&harness(), input_reader).unwrap();
    let flags = io_state
        .get_particle_object_mut("b")
        .unwrap()
        .view_attr_mut::<attribute::Flags>()
        .unwrap();
    flags[2] = u32_to_two_f32((ParticleFlags::IS_SOLID | ParticleFlags::FAILED).bits());
    io_state
}

fn view<A: attribute::Trait>(io_state: &IoState, object: &str) -> Vec<A::T>
where
    A::T: Clone,
{
    io_state
        .get_particle_object(object)
        .unwrap()
        .view_attr::<A>()
        .unwrap()
        .to_vec()
}

fn assert_close<const N: usize>(found: &[[f32; N]], expected: &[[f32; N]], what: &str) {
    assert_eq!(found.len(), expected.len(), "{what}");
    for (found, expected) in found.iter().zip(expected) {
        for (f, e) in found.iter().zip(expected) {
            assert!(
                (f - e).abs() <= 1e-5 * e.abs().max(1.),
                "{what}: {found:?} vs {expected:?}"
            );
        }
    }
}

fn assert_overwritten(io_state: &IoState, backend: &str) {
    let flags: Vec<ParticleFlags> = view::<attribute::Flags>(io_state, "b")
        .into_iter()
        .map(|flags| ParticleFlags(two_f32_to_u32(flags)))
        .collect();
    assert_eq!(flags, OVERWRITTEN_FLAGS, "{backend} flags");

    let untouched: Vec<ParticleFlags> = view::<attribute::Flags>(io_state, "a")
        .into_iter()
        .map(|flags| ParticleFlags(two_f32_to_u32(flags)))
        .collect();
    assert_eq!(
        untouched,
        vec![ParticleFlags::IS_SOLID | ParticleFlags::IS_ACTIVE; 2],
        "{backend} flags of the other object"
    );

    assert_close(
        &view::<attribute::Velocity>(io_state, "b"),
        &VELOCITIES,
        &format!("{backend} velocity"),
    );
    assert_close(
        &view::<attribute::GoalPosition>(io_state, "b"),
        &GOAL_POSITIONS,
        &format!("{backend} goal"),
    );
    assert_close(
        &view::<attribute::InitialPosition>(io_state, "b"),
        &INITIAL_POSITIONS,
        &format!("{backend} initial position"),
    );
    assert_close(
        &view::<attribute::Position>(io_state, "b"),
        &POSITIONS,
        &format!("{backend} position"),
    );
    for (key, column) in [
        view::<attribute::PositionGradientX>(io_state, "b"),
        view::<attribute::PositionGradientY>(io_state, "b"),
        view::<attribute::PositionGradientZ>(io_state, "b"),
    ]
    .into_iter()
    .zip(GRADIENT_COLUMNS)
    {
        assert_close(&key, &[column; 3], &format!("{backend} position gradient"));
    }

    let scalars = |values: Vec<f32>| values.into_iter().map(|v| [v]).collect::<Vec<_>>();
    assert_close(
        &scalars(view::<attribute::Density>(io_state, "b")),
        &DENSITIES.map(|v| [v]),
        &format!("{backend} density"),
    );
    assert_close(
        &scalars(view::<attribute::InitialSize>(io_state, "b")),
        &SIZES.map(|v| [v]),
        &format!("{backend} size"),
    );
    assert_close(
        &scalars(view::<attribute::YoungsModulus>(io_state, "b")),
        &YOUNGS_MODULI.map(|v| [v]),
        &format!("{backend} youngs modulus"),
    );
    assert_close(
        &scalars(view::<attribute::Exponent>(io_state, "b")),
        &EXPONENTS.map(|v| [v as f32]),
        &format!("{backend} exponent"),
    );
    for (attribute, values) in [
        (
            view::<attribute::PoissonsRatio>(io_state, "b"),
            POISSONS_RATIOS,
        ),
        (
            view::<attribute::ViscosityDynamic>(io_state, "b"),
            VISCOSITIES_DYNAMIC,
        ),
        (
            view::<attribute::ViscosityBulk>(io_state, "b"),
            VISCOSITIES_BULK,
        ),
        (view::<attribute::BulkModulus>(io_state, "b"), BULK_MODULI),
        (view::<attribute::SandAlpha>(io_state, "b"), SAND_ALPHAS),
    ] {
        assert_close(
            &scalars(attribute),
            &values.map(|v| [v]),
            &format!("{backend} parameters {values:?}"),
        );
    }
    let collider_bits: Vec<u32> = view::<attribute::ColliderBits>(io_state, "b")
        .into_iter()
        .map(two_f32_to_u32)
        .collect();
    assert_eq!(collider_bits, COLLIDER_BITS, "{backend} collider bits");
}

fn assert_pulled_towards_goal(io_state: &IoState, backend: &str) {
    let [velocity, ..] = view::<attribute::Velocity>(io_state, "b")[..] else {
        unreachable!()
    };
    assert!(
        velocity[0] > VELOCITIES[0][0] + 2.,
        "{backend}: {velocity:?} is not pulled towards the goal at {:?}",
        GOAL_POSITIONS[0]
    );
}

#[test]
fn cpu_overwrites_attributes_at_the_frame_boundary() {
    let directory = TempDir::new().unwrap();
    let mut input_reader = record(&directory.path().join("input.bin"));
    let io_state = initial_state(&mut input_reader);
    let mut cpu_state = CpuState::new(0, io_state, input_reader).unwrap();

    let mut next_state = || {
        let (io_state, result) = cpu_state
            .produce_next_state(
                &harness(),
                CpuRunParameters {
                    max_time_step: 1.,
                    adaptive_time_steps: true,
                    store_bvh: false,
                    store_grid: false,
                },
            )
            .unwrap();
        result.unwrap();
        io_state
    };

    assert_overwritten(&next_state(), "cpu");
    assert_pulled_towards_goal(&next_state(), "cpu");
}

#[test]
fn gpu_overwrites_attributes_at_the_frame_boundary() {
    let directory = TempDir::new().unwrap();
    let mut input_reader = record(&directory.path().join("input.bin"));
    let io_state = initial_state(&mut input_reader);
    let harness = harness();
    let gpu = GpuContext::available_gpus().into_iter().next().unwrap();
    let mut gpu_state = GpuState::new(0, io_state, input_reader, gpu, &harness, 1., None).unwrap();

    let mut next_state = || {
        let (io_state, result) = gpu_state
            .produce_next_state(
                &harness,
                GpuRunParameters {
                    adaptive_time_steps: true,
                    store_bvh: false,
                    store_grid: false,
                },
            )
            .unwrap();
        result.unwrap();
        io_state
    };

    assert_overwritten(&next_state(), "gpu");
    assert_pulled_towards_goal(&next_state(), "gpu");
}
