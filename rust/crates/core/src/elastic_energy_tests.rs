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

use std::{borrow::Cow, num::NonZero};

use nalgebra::Matrix3;
use squishy_volumes_api::InputBulk;
use squishy_volumes_cpu::{CpuRunParameters, CpuState};
use squishy_volumes_file_frame::{IoState, combine_matrices, two_f32_to_u32};
use squishy_volumes_file_input::{
    BulkAttribute::Particles, FrameBulk, FrameBulkMeta, FrameBulkParticles as P, InputHeader,
    InputObject, InputObjectParticles, InputReader, write_test_input_frames,
};
use squishy_volumes_gpu::{GpuContext, GpuRunParameters, GpuState};
use squishy_volumes_util::{
    AnimatedGlobals, ParticleFlags as F, ParticleParameters, attribute as a,
    elastic_energy_inviscid, elastic_energy_neo_hookean,
};
use squishy_volumes_xpu::Harness;
use tempfile::TempDir;

use crate::initialization::initialize_io_state;

const SOLID: F = F::IS_SOLID.union(F::IS_ACTIVE);
const INACTIVE: F = F::IS_SOLID;
const A: &[(F, [f32; 3])] = &[
    (SOLID, [1.2, 1., 1.]),
    (SOLID, [1., 0.9, 1.05]),
    (INACTIVE, [0.8, 0.9, 1.]),
];
const B: &[(F, [f32; 3])] = &[
    (F::IS_FLUID.union(F::IS_ACTIVE), [0.9, 0.95, 0.9]),
    (INACTIVE, [-1., 1., 1.]),
];

fn particle_bulks(
    object: &str,
    offset: f32,
    particles: &[(F, [f32; 3])],
) -> [FrameBulk<'static>; 2] {
    let mask = (F::IS_SOLID | F::IS_FLUID | F::IS_ACTIVE).bits() as i32;
    let flags = particles
        .iter()
        .flat_map(|(flags, _)| [mask, flags.bits() as i32]);
    let transforms = particles
        .iter()
        .enumerate()
        .flat_map(|(i, (_, [x, y, z]))| {
            let position = [offset + 4. * i as f32, 0., 0., 1.];
            [
                [*x, 0., 0., 0.],
                [0., *y, 0., 0.],
                [0., 0., *z, 0.],
                position,
            ]
            .concat()
        });
    let meta = |attribute| FrameBulkMeta {
        object_name: object.to_string(),
        captured_attribute: Particles(attribute),
    };
    [
        FrameBulk {
            meta: meta(P::Flags),
            data: InputBulk::Ints(flags.collect::<Vec<_>>().into()),
        },
        FrameBulk {
            meta: meta(P::Transform),
            data: InputBulk::Floats(Cow::Owned(transforms.collect())),
        },
    ]
}

fn record(directory: &TempDir) -> (IoState, InputReader) {
    let objects = [("a", A), ("b", B)].map(|(object, particles)| {
        let particles = InputObjectParticles {
            num_particles: particles.len() as u32,
            alembic_attributes: Default::default(),
        };
        (object.to_string(), InputObject::Particles(particles))
    });
    let first_frame = [particle_bulks("a", 0., A), particle_bulks("b", 20., B)].concat();
    let globals = AnimatedGlobals {
        gravity_z: 0.,
        ..Default::default()
    };
    let frames = [first_frame, vec![], vec![]].map(|bulks| (globals, bulks));
    let path = directory.path().join("input.bin");
    let mut input_reader = write_test_input_frames(path, InputHeader::test_input(objects), frames);
    (
        initialize_io_state(&harness(), &mut input_reader).unwrap(),
        input_reader,
    )
}

fn assert_stored_energies(backend: &str, mut produce: impl FnMut() -> IoState) {
    for frame in 0..2 {
        let mut io_state = produce();
        let gradients: Vec<_> = combine_matrices(
            io_state.collect::<a::PositionGradientX>().unwrap(),
            io_state.collect::<a::PositionGradientY>().unwrap(),
            io_state.collect::<a::PositionGradientZ>().unwrap(),
        )
        .collect();
        let flags: Vec<_> = io_state.collect::<a::Flags>().unwrap().collect();
        let youngs_moduli: Vec<_> = io_state.collect::<a::YoungsModulus>().unwrap().collect();
        let poissons_ratios: Vec<_> = io_state.collect::<a::PoissonsRatio>().unwrap().collect();
        let bulk_moduli: Vec<_> = io_state.collect::<a::BulkModulus>().unwrap().collect();
        let exponents: Vec<_> = io_state.collect::<a::Exponent>().unwrap().collect();
        let energies: Vec<_> = io_state.collect::<a::ElasticEnergy>().unwrap().collect();
        assert_eq!(energies.len(), A.len() + B.len());
        for (i, stored) in energies.into_iter().enumerate() {
            let gradient = Matrix3::from(gradients[i]);
            let parameters = ParticleParameters {
                youngs_modulus: youngs_moduli[i],
                poissons_ratio: poissons_ratios[i],
                ..Default::default()
            };
            let expected = if gradient.determinant() <= 0. {
                0.
            } else if F(two_f32_to_u32(flags[i])).contains(F::IS_SOLID) {
                elastic_energy_neo_hookean(parameters.mu(), parameters.lambda(), &gradient)
            } else {
                elastic_energy_inviscid(bulk_moduli[i], exponents[i] as i32, &gradient)
            };
            assert!(
                (stored - expected).abs() <= 1e-3 * expected.abs(),
                "{backend} frame {frame}, particle {i}: stored {stored}, expected {expected}"
            );
        }
    }
}

fn harness() -> Harness {
    Harness::new("test".to_string(), NonZero::new(1).unwrap())
}

#[test]
fn cpu_and_gpu_store_the_elastic_energy_of_the_stored_state() {
    let directory = TempDir::new().unwrap();
    let (io_state, input_reader) = record(&directory);
    let mut cpu = CpuState::new(0, io_state, input_reader).unwrap();
    assert_stored_energies("cpu", || {
        let parameters = CpuRunParameters {
            max_time_step: 1.,
            adaptive_time_steps: true,
            store_bvh: false,
            store_grid: false,
        };
        let (io_state, result) = cpu.produce_next_state(&harness(), parameters).unwrap();
        result.map(|()| io_state).unwrap()
    });

    let (io_state, input_reader) = record(&directory);
    let device = GpuContext::available_gpus().into_iter().next().unwrap();
    let mut gpu = GpuState::new(0, io_state, input_reader, device, &harness(), 1., None).unwrap();
    assert_stored_energies("gpu", || {
        let parameters = GpuRunParameters {
            adaptive_time_steps: true,
            store_bvh: false,
            store_grid: false,
        };
        let (io_state, result) = gpu.produce_next_state(&harness(), parameters).unwrap();
        result.map(|()| io_state).unwrap()
    });
}
