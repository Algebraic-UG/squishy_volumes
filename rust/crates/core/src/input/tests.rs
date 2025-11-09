// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    fs::OpenOptions,
    io::{Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use tempfile::{Builder, TempDir};

use super::{
    BulkData, InputFrame, InputHeader, InputReader, InputWriter,
    common::{InputError, MAGIC_LEN},
};

fn test_file() -> (PathBuf, TempDir) {
    let tmp_dir = Builder::new()
        .prefix("SquishyVolumesTestDir")
        .tempdir()
        .unwrap();
    (tmp_dir.path().join("test_input.bin"), tmp_dir)
}

fn test_header() -> InputHeader {
    InputHeader {
        test_param_a: "foo".to_string(),
        test_param_b: "bar".to_string(),
        test_param_c: "car".to_string(),
    }
}

fn test_frames() -> Vec<InputFrame> {
    vec![
        InputFrame {
            bulk: Default::default(),
        },
        InputFrame {
            bulk: [(
                "Some float attribute".to_string(),
                BulkData::F32(vec![1., 2., 3., 42.]),
            )]
            .into_iter()
            .collect(),
        },
        InputFrame {
            bulk: Default::default(),
        },
        InputFrame {
            bulk: [(
                "Some int attribute".to_string(),
                BulkData::I32(vec![1, 2, 3, 42]),
            )]
            .into_iter()
            .collect(),
        },
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
    InputWriter::new(path, test_header()).unwrap();
}

#[test]
fn test_write_partial() {
    let (path, _guard) = test_file();
    let mut writer = InputWriter::new(path, test_header()).unwrap();
    for frame in test_frames().into_iter().take(2) {
        writer.record_frame(frame).unwrap();
    }
}

fn write_full<P: AsRef<Path>>(path: P) {
    let mut writer = InputWriter::new(path, test_header()).unwrap();
    for frame in test_frames() {
        writer.record_frame(frame).unwrap();
    }
    writer.flush().unwrap();
}

#[test]
fn test_write_full() {
    let (path, _guard) = test_file();
    write_full(path);
}

#[test]
fn test_read_start() {
    let (path, _guard) = test_file();
    write_full(&path);
    InputReader::new(path).unwrap();
}

#[test]
fn test_read_header() {
    let (path, _guard) = test_file();
    write_full(&path);
    let mut reader = InputReader::new(path).unwrap();
    assert!(test_header() == reader.read_header().unwrap());
}

#[test]
fn test_read_header_and_random_frames() {
    let (path, _guard) = test_file();
    write_full(&path);
    let mut reader = InputReader::new(path).unwrap();
    assert!(test_header() == reader.read_header().unwrap());

    let mut rng = SmallRng::seed_from_u64(42);
    let mut frames: Vec<_> = test_frames().into_iter().enumerate().collect();
    frames.shuffle(&mut rng);

    for (idx, frame) in frames {
        assert!(frame == reader.read_frame(idx).unwrap());
    }
}

#[test]
fn test_wrong_magic_number() {
    let (path, _guard) = test_file();
    write_full(&path);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::Start(5)).unwrap();
        let _ = f.write(&[1, 2, 3, 4]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::MagicMismatch)
    ));
}

#[test]
fn test_wrong_version() {
    let (path, _guard) = test_file();
    write_full(&path);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::Start((MAGIC_LEN + 5).try_into().unwrap()))
            .unwrap();
        let _ = f.write(&[1, 2, 3, 4]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::VersionMismatch(_))
    ));
}

#[test]
fn test_frame_not_available() {
    let (path, _guard) = test_file();
    write_full(&path);
    let mut reader = InputReader::new(path).unwrap();
    assert!(matches!(
        reader.read_frame(test_frames().len()),
        Err(InputError::FrameNotAvailable { .. },)
    ));
}

#[test]
fn test_index_offset_mishap_io() {
    let (path, _guard) = test_file();
    write_full(&path);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::End(-8)).unwrap();
        let _ = f.write(&[u8::MAX; 8]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::OffsetReading(
            crate::input::common::InputOffsetReadingError::IoError(_)
        ))
    ));
}

#[test]
fn test_index_offset_mishap_bincode() {
    let (path, _guard) = test_file();
    write_full(&path);
    {
        let mut f = OpenOptions::new().write(true).open(&path).unwrap();
        f.seek(SeekFrom::End(-8)).unwrap();
        let _ = f.write(&[0; 8]).unwrap();
    }
    assert!(matches!(
        InputReader::new(path),
        Err(InputError::OffsetReading(
            crate::input::common::InputOffsetReadingError::BincodeError(_)
        ))
    ));
}
