// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fmt,
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use squishy_volumes_api::InputBulk;
use squishy_volumes_util::AnimatedGlobals;
use tempfile::{Builder, TempDir};

use crate::{
    FrameBulk, InputConsts, InputError, InputObject, InputObjectCollider, InputObjectParticles,
    InputOffsetReadingError, OwnedFrameBulk,
};

use super::{InputFrame, InputHeader, InputReader, InputWriter};

fn test_file() -> (PathBuf, TempDir) {
    let tmp_dir = Builder::new()
        .prefix("SquishyVolumesTestDir")
        .tempdir()
        .unwrap();
    (tmp_dir.path().join("test_input.bin"), tmp_dir)
}

fn test_header(num_particles: u32, num_vertices: u32, num_triangles: u32) -> InputHeader {
    let consts = InputConsts::test_input();
    let objects = [
        (
            "foo".to_string(),
            InputObject::Particles(InputObjectParticles { num_particles }),
        ),
        (
            "bar".to_string(),
            InputObject::Particles(InputObjectParticles { num_particles }),
        ),
        (
            "car".to_string(),
            InputObject::Collider(InputObjectCollider {
                collider_id: 0,
                num_vertices,
                num_triangles,
            }),
        ),
    ]
    .into_iter()
    .collect();

    InputHeader { consts, objects }
}

fn test_frames(
    num_particles: u32,
    num_vertices: u32,
    num_triangles: u32,
) -> Vec<(AnimatedGlobals, Vec<FrameBulk<'static>>)> {
    vec![
        InputFrame::test_input_0(num_particles, num_vertices, num_triangles),
        InputFrame::test_input_0(num_particles, num_vertices, num_triangles),
        InputFrame::test_input_0(num_particles, num_vertices, num_triangles),
    ]
}

#[test]
fn test_creating_tempdir() {
    let dir;
    {
        let (path, _guard) = test_file();
        dir = path.parent().unwrap().to_path_buf();
        assert!(dir.exists());
    }
    assert!(!dir.exists());
}

#[test]
fn test_write_start() {
    let (path, _guard) = test_file();
    InputWriter::new(path, test_header(100, 99, 33)).unwrap();
}

#[test]
fn test_write_partial() {
    let (path, _guard) = test_file();
    let mut writer = InputWriter::new(path, test_header(100, 99, 33)).unwrap();
    for (animated_globals, bulk) in test_frames(100, 99, 33).iter().take(2) {
        writer.start_frame(animated_globals).unwrap();
        for bulk in bulk {
            writer.record_bulk(bulk).unwrap();
        }
    }
}

fn write_full<P: AsRef<Path> + fmt::Debug>(
    path: P,
    num_particles: u32,
    num_vertices: u32,
    num_triangles: u32,
) {
    let mut writer = InputWriter::new(
        path,
        test_header(num_particles, num_vertices, num_triangles),
    )
    .unwrap();
    for (animated_globals, bulk) in &test_frames(num_particles, num_vertices, num_triangles) {
        writer.start_frame(animated_globals).unwrap();
        for bulk in bulk {
            writer.record_bulk(bulk).unwrap();
        }
    }
    writer.flush().unwrap();
}

#[test]
fn test_write_full() {
    let (path, _guard) = test_file();
    write_full(path, 10, 9, 3);
}

#[test]
fn test_read_start() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    InputReader::new(path).unwrap();
}

#[test]
fn test_read_header_and_random_frames() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    let mut reader = InputReader::new(path).unwrap();
    assert_eq!(test_header(10, 9, 3), *reader.header());

    let mut rng = SmallRng::seed_from_u64(42);
    let mut frames: Vec<_> = test_frames(10, 9, 3).into_iter().enumerate().collect();
    frames.shuffle(&mut rng);

    for (idx, (animated_globals, bulk)) in frames {
        let frame = reader.read_frame(idx).unwrap();
        assert_eq!(animated_globals, frame.animated_globals);
        let bulk: Vec<OwnedFrameBulk> = bulk.into_iter().map(Into::into).collect();
        assert_eq!(bulk, frame.bulk);
    }
}

#[test]
fn test_wrong_magic_number() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::Start(5)).unwrap();
        let _ = f.write(&[1, 2, 3, 4]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::FileUtil(
            squishy_volumes_file_util::Error::MagicMismatch { .. }
        ))
    ));
}

#[test]
fn test_wrong_version() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::Start(
            (squishy_volumes_file_util::MAGIC_LEN + 5)
                .try_into()
                .unwrap(),
        ))
        .unwrap();
        let _ = f.write(&[1, 2, 3, 4]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::FileUtil(
            squishy_volumes_file_util::Error::VersionMismatch { .. }
        ))
    ));
}

#[test]
fn test_frame_not_available() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    let mut reader = InputReader::new(path).unwrap();
    assert!(matches!(
        reader.read_frame(test_frames(10, 9, 3).len()),
        Err(InputError::FrameNotAvailable { .. },)
    ));
}

#[test]
fn test_index_offset_mishap_io() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::End(-8)).unwrap();
        let _ = f.write(&[u8::MAX; 8]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::OffsetReading(InputOffsetReadingError::IoError(
            _
        )))
    ));
}

#[test]
fn test_index_offset_mishap_bincode() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::End(-8)).unwrap();
        let _ = f.write(&[0; 8]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::OffsetReading(
            InputOffsetReadingError::BincodeError(_)
        ))
    ));
}

#[test]
fn test_length_mismatch() {
    let (path, _guard) = test_file();
    let mut writer = InputWriter::new(path, test_header(10, 9, 3)).unwrap();

    let (animated_globals, bulk) = InputFrame::test_input_0(1, 2, 3);
    writer.start_frame(&animated_globals).unwrap();
    assert!(matches!(
        writer.record_bulk(&bulk[0]),
        Err(InputError::FrameVerifcationError {
            error: crate::FrameVerifcationError::ObjectError {
                error: crate::ObjectError::AttributeError {
                    error: crate::AttributeError::LengthMismatch { .. },
                    ..
                },
                ..
            },
            ..
        }),
    ));
}

#[test]
fn test_object_changed_type() {
    let (path, _guard) = test_file();
    let mut writer = InputWriter::new(path, test_header(10, 9, 3)).unwrap();
    let (animated_globals, _) = InputFrame::test_input_0(10, 9, 3);
    let bulk = FrameBulk {
        meta: crate::FrameBulkMeta {
            object_name: "foo".to_string(),
            captured_attribute: crate::BulkAttribute::Collider(
                crate::FrameBulkCollider::VertexPositions,
            ),
        },
        data: InputBulk::Floats(Default::default()),
    };
    writer.start_frame(&animated_globals).unwrap();
    assert!(matches!(
        writer.record_bulk(&bulk),
        Err(InputError::FrameVerifcationError {
            error: crate::FrameVerifcationError::ObjectError {
                error: crate::ObjectError::ObjectChangedType,
                ..
            },
            ..
        }),
    ));
}

#[test]
fn test_object_not_in_header() {
    let (path, _guard) = test_file();
    let mut writer = InputWriter::new(path, test_header(10, 9, 3)).unwrap();
    let (animated_globals, _) = InputFrame::test_input_0(10, 9, 3);
    let bulk = FrameBulk {
        meta: crate::FrameBulkMeta {
            object_name: "newfoo".to_string(),
            captured_attribute: crate::BulkAttribute::Collider(
                crate::FrameBulkCollider::VertexPositions,
            ),
        },
        data: InputBulk::Floats(Default::default()),
    };
    writer.start_frame(&animated_globals).unwrap();
    assert!(matches!(
        writer.record_bulk(&bulk),
        Err(InputError::FrameVerifcationError {
            error: crate::FrameVerifcationError::ObjectError {
                error: crate::ObjectError::ObjectNotInHeader,
                ..
            },
            ..
        }),
    ));
}
