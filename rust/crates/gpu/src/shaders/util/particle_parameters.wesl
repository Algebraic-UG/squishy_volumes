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

struct ParticleParameters {
    density: f32,
    initial_size: f32,
    viscosity_dynamic: f32,
    viscosity_bulk: f32,
    youngs_modulus: f32,
    poissons_ratio: f32,
    sand_alpha: f32,
    bulk_modulus: f32,
    exponent: i32,
}

fn initial_volume(p: ParticleParameters) -> f32 {
    return p.initial_size * p.initial_size * p.initial_size;
}

fn mass(p: ParticleParameters) -> f32 {
    return p.density * initial_volume(p);
}

fn mu(p: ParticleParameters) -> f32 {
    return p.youngs_modulus / 2. / (1. + p.poissons_ratio);
}

fn lambda(p: ParticleParameters) -> f32 {
    return p.youngs_modulus * p.poissons_ratio
            / (1. + p.poissons_ratio)
            / (1. - 2. * p.poissons_ratio);
}
