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

use std::path::Path;

use alembic_rust::AttributeType;
use squishy_volumes_util::attribute;
use strum::IntoEnumIterator as _;

use crate::*;

const NUM_PARTICLES: usize = 3;

fn attribute_value(key: attribute::Key) -> AttributeValue {
    let ramp = (0..NUM_PARTICLES).map(|i| i as f32);
    match key.alembic_type() {
        AttributeType::Float1 => AttributeValue::Float1(ramp.collect()),
        AttributeType::Float2 => AttributeValue::Float2(ramp.map(|v| [v, -v]).collect()),
        AttributeType::Float3 => AttributeValue::Float3(ramp.map(|v| [v, -v, 2. * v]).collect()),
    }
}

fn small_frame() -> Frame {
    let particles = ParticlesObject {
        num_particles: NUM_PARTICLES,
        alembic_attributes: attribute::Key::iter().collect(),
        attributes: attribute::Key::iter()
            .map(|key| (key, attribute_value(key)))
            .collect(),
    };
    let mut frame = Frame::default();
    frame.io_state.time = 1.5;
    frame
        .io_state
        .particle_objects
        .insert("cube".to_string(), particles);
    frame.error_chain = Some(vec!["outer".to_string(), "inner".to_string()]);
    frame
}

fn read_error(path: &Path) -> ReadError {
    match Frame::read(path) {
        Err(Error::Read {
            path: error_path,
            source,
        }) => {
            assert_eq!(error_path, path);
            source
        }
        Err(Error::Write { .. }) => panic!("reading must not fail with a write error"),
        Ok(_) => panic!("reading '{}' must fail", path.display()),
    }
}

#[test]
fn round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("frame.bin");

    let size = small_frame().write(&path).unwrap();
    assert_eq!(size, std::fs::metadata(&path).unwrap().len());

    let frame = Frame::read(&path).unwrap();
    assert_eq!(frame.io_state.time, 1.5);
    assert_eq!(
        frame.error_chain,
        Some(vec!["outer".to_string(), "inner".to_string()])
    );
    let particles = frame.io_state.get_particle_object("cube").unwrap();
    assert_eq!(particles.num_particles, NUM_PARTICLES);
    for key in attribute::Key::iter() {
        assert_eq!(
            format!("{:?}", particles.attributes[&key]),
            format!("{:?}", attribute_value(key)),
            "{key:?}"
        );
    }
}

#[test]
fn reading_a_missing_file_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.bin");

    assert!(matches!(
        read_error(&path),
        ReadError::Open { source } if source.kind() == std::io::ErrorKind::NotFound
    ));
}

#[test]
fn reading_garbage_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("garbage.bin");
    std::fs::write(&path, b"this is not an alembic archive").unwrap();

    assert!(matches!(read_error(&path), ReadError::Archive { .. }));
}

#[test]
fn reading_a_truncated_file_names_it() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("frame.bin");
    small_frame().write(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, &bytes[..bytes.len() / 2]).unwrap();

    assert!(matches!(read_error(&path), ReadError::Archive { .. }));
}

#[test]
fn reading_a_directory_names_it() {
    let dir = tempfile::tempdir().unwrap();

    assert!(matches!(
        read_error(dir.path()),
        ReadError::Open { .. } | ReadError::Archive { .. }
    ));
}

#[test]
fn writing_below_a_file_names_the_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let parent = dir.path().join("not_a_directory");
    std::fs::write(&parent, b"").unwrap();

    let result = small_frame().write(parent.join("frame.bin"));
    assert!(matches!(
        result,
        Err(Error::Write { path, source: WriteError::Create { .. } })
            if path == parent.join("temp.bin")
    ));
}
