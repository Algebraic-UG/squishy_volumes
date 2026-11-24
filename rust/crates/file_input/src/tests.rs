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

use std::{
    fmt,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use squishy_volumes_api::InputBulk;
use squishy_volumes_util::AnimatedGlobals;
use tempfile::{Builder, TempDir};

use crate::{
    Error, FileError, FrameBulk, Header, InputConsts, InputObject, InputObjectCollider,
    InputObjectParticles, OwnedFrameBulk,
};

use super::{InputFrame, InputHeader, InputReader, InputWriter};

fn test_file() -> (PathBuf, TempDir) {
    let tmp_dir = Builder::new()
        .prefix("SquishyVolumesTestDir")
        .tempdir()
        .unwrap();
    (tmp_dir.path().join("test_input.bin"), tmp_dir)
}

fn test_header(num_particles: u32, num_vertices: u32, num_triangles: u32) -> Header {
    let consts = InputConsts::test_input();
    let objects = [
        (
            "foo".to_string(),
            InputObject::Particles(InputObjectParticles {
                num_particles,
                alembic_attributes: Default::default(),
            }),
        ),
        (
            "bar".to_string(),
            InputObject::Particles(InputObjectParticles {
                num_particles,
                alembic_attributes: Default::default(),
            }),
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

    Header::new(InputHeader { consts, objects })
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
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadMagicAndVersion { source: squishy_volumes_file_util::Error::MagicMismatch { .. } } }) if error_path == path
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
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadMagicAndVersion { source: squishy_volumes_file_util::Error::VersionMismatch { .. } } }) if error_path == path
    ));
}

#[test]
fn test_frame_not_available() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    let mut reader = InputReader::new(path).unwrap();
    assert!(matches!(
        reader.read_frame(test_frames(10, 9, 3).len()),
        Err(Error::FrameNotAvailable { .. },)
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
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadOffsets { source } })
            if error_path == path && matches!(*source, bincode::ErrorKind::Io(_))
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
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadOffsets { .. } }) if error_path == path
    ));
}

fn overwrite_at(path: &Path, pos: SeekFrom, bytes: &[u8]) {
    let mut f = OpenOptions::new().write(true).open(path).unwrap();
    f.seek(pos).unwrap();
    f.write_all(bytes).unwrap();
}

fn index_offset(path: &Path) -> u64 {
    let mut f = File::open(path).unwrap();
    f.seek(SeekFrom::End(-8)).unwrap();
    let mut bytes = [0; 8];
    f.read_exact(&mut bytes).unwrap();
    u64::from_le_bytes(bytes)
}

#[test]
fn test_missing_input_file() {
    let (path, _guard) = test_file();
    assert!(matches!(
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::Open { source } })
            if error_path == path && source.kind() == std::io::ErrorKind::NotFound
    ));
}

#[test]
fn test_create_in_missing_directory() {
    let (path, _guard) = test_file();
    let path = path.join("nested").join("input.bin");
    assert!(matches!(
        InputWriter::new(&path, test_header(10, 9, 3)),
        Err(Error::File { path: error_path, source: FileError::Create { .. } }) if error_path == path
    ));
}

#[test]
fn test_truncated_version() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(
            (squishy_volumes_file_util::MAGIC_LEN + 3)
                .try_into()
                .unwrap(),
        )
        .unwrap();
    assert!(matches!(
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadMagicAndVersion { source: squishy_volumes_file_util::Error::ReadVersion(_) } }) if error_path == path
    ));
}

#[test]
fn test_header_object_name_past_end_of_file() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    let object_count_pos =
        squishy_volumes_file_util::DATA_OFFSET + size_of::<InputConsts>() + 3 * size_of::<usize>();
    let first_object_name_len_pos = object_count_pos + size_of::<u64>();
    overwrite_at(
        &path,
        SeekFrom::Start(first_object_name_len_pos.try_into().unwrap()),
        &1_000_000u64.to_le_bytes(),
    );
    assert!(matches!(
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadHeader { source } })
            if error_path == path && matches!(&*source, bincode::ErrorKind::Io(io) if io.kind() == std::io::ErrorKind::UnexpectedEof)
    ));
}

#[test]
fn test_missing_offsets() {
    let (path, _guard) = test_file();
    let mut writer = InputWriter::new(&path, test_header(10, 9, 3)).unwrap();
    for (animated_globals, bulk) in &test_frames(10, 9, 3) {
        writer.start_frame(animated_globals).unwrap();
        for bulk in bulk {
            writer.record_bulk(bulk).unwrap();
        }
    }
    drop(writer);
    assert!(matches!(
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadOffsets { .. } }) if error_path == path
    ));
}

#[test]
fn test_truncated_offsets() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    let file_len = std::fs::metadata(&path).unwrap().len();
    overwrite_at(&path, SeekFrom::End(-8), &(file_len - 4).to_le_bytes());
    assert!(matches!(
        InputReader::new(&path),
        Err(Error::File { path: error_path, source: FileError::ReadOffsets { source } })
            if error_path == path && matches!(&*source, bincode::ErrorKind::Io(io) if io.kind() == std::io::ErrorKind::UnexpectedEof)
    ));
}

#[test]
fn test_frame_truncated_mid_bulk() {
    let (path, _guard) = test_file();
    write_full(&path, 10, 9, 3);
    let (_, bulk) = InputFrame::test_input_0(10, 9, 3);
    let last_bulk = bulk.last().unwrap();
    let element_size = match last_bulk.data {
        InputBulk::Bool(_) => 1,
        InputBulk::Floats(_) | InputBulk::Ints(_) => 4,
    };
    let data_len = u64::try_from(last_bulk.data.len()).unwrap();
    let length_prefix_pos = index_offset(&path) - element_size * data_len - 8;
    overwrite_at(
        &path,
        SeekFrom::Start(length_prefix_pos),
        &(data_len + 1_000_000).to_le_bytes(),
    );

    let mut reader = InputReader::new(&path).unwrap();
    let last_frame = reader.len() - 1;
    assert!(matches!(
        reader.read_frame(last_frame),
        Err(Error::File { path: error_path, source: FileError::ReadFrame { frame, source } })
            if error_path == path && frame == last_frame && matches!(&*source, bincode::ErrorKind::Io(io) if io.kind() == std::io::ErrorKind::UnexpectedEof)
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
        Err(Error::FrameVerification {
            source: crate::FrameVerificationError::Object {
                source: crate::ObjectError::Attribute {
                    source: crate::BulkAttributeError::LengthMismatch { .. },
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
            captured_attribute: crate::BulkAttribute::Collider(crate::FrameBulkCollider::Position),
        },
        data: InputBulk::Floats(Default::default()),
    };
    writer.start_frame(&animated_globals).unwrap();
    assert!(matches!(
        writer.record_bulk(&bulk),
        Err(Error::FrameVerification {
            source: crate::FrameVerificationError::Object {
                source: crate::ObjectError::ChangedType,
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
            captured_attribute: crate::BulkAttribute::Collider(crate::FrameBulkCollider::Position),
        },
        data: InputBulk::Floats(Default::default()),
    };
    writer.start_frame(&animated_globals).unwrap();
    assert!(matches!(
        writer.record_bulk(&bulk),
        Err(Error::FrameVerification {
            source: crate::FrameVerificationError::Object {
                source: crate::ObjectError::NotInHeader,
                ..
            },
            ..
        }),
    ));
}
