// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::path::{Path, PathBuf};

use rand::{SeedableRng, rngs::SmallRng, seq::SliceRandom};
use tempfile::{Builder, TempDir};

use crate::input::{InputFrame, InputHeader, InputReader, InputWriter};

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
            test_data: Default::default(),
        },
        InputFrame {
            test_data: vec![1., 2., 3., 42.],
        },
        InputFrame {
            test_data: Default::default(),
        },
        InputFrame {
            test_data: vec![1., 2., 3., 42.],
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
