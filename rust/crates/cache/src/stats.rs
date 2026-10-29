// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::io::Write;

use super::*;

#[derive(Clone, Default)]
pub struct Stats {
    pub grid_node_count: Option<u32>,
    pub compute: Option<ComputeStats>,
}

#[derive(Clone, Default)]
pub struct ComputeStats {
    pub remaining_time_sec: f32,
    pub last_frame_time_sec: f32,
    pub last_frame_substeps: u32,
}

// the python side parses this, keep in sync
const STATS_HEADER: &str =
    "frame,grid_node_count,last_frame_time_sec,remaining_time_sec,last_frame_substeps\n";

fn format_optional(value: Option<impl std::fmt::Display>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}

fn stats_row(frame_index: usize, stats: &Stats) -> String {
    let compute = stats.compute.as_ref();
    format!(
        "{frame_index},{},{},{},{}\n",
        format_optional(stats.grid_node_count),
        format_optional(compute.map(|compute| compute.last_frame_time_sec)),
        format_optional(compute.map(|compute| compute.remaining_time_sec)),
        format_optional(compute.map(|compute| compute.last_frame_substeps)),
    )
}

pub fn append_stats(
    cache_dir: impl AsRef<std::path::Path>,
    frame_index: usize,
    stats: &Stats,
) -> Result<(), std::io::Error> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(stats_path(cache_dir))?;

    let mut content = String::new();
    if file.metadata()?.len() == 0 {
        content.push_str(STATS_HEADER);
    }
    content.push_str(&stats_row(frame_index, stats));

    // single write, so readers never see a partial row (in practice)
    file.write_all(content.as_bytes())
}

// rows are appended in frame order, so dropping frames is cutting off a suffix
pub fn truncate_stats(
    cache_dir: impl AsRef<std::path::Path>,
    from_frame: usize,
) -> Result<(), std::io::Error> {
    let path = stats_path(cache_dir);
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };

    let mut keep = 0;
    for (i, line) in content.split_inclusive('\n').enumerate() {
        if !line.ends_with('\n') {
            break;
        }
        if i > 0 {
            let frame = line
                .split(',')
                .next()
                .and_then(|frame| frame.parse::<usize>().ok());
            if frame.is_none_or(|frame| frame >= from_frame) {
                break;
            }
        }
        keep += line.len();
    }

    if keep < content.len() {
        std::fs::OpenOptions::new()
            .write(true)
            .open(&path)?
            .set_len(keep as u64)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "squishy_volumes_stats_test_{name}_{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn rows(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_to_string(stats_path(dir))
            .unwrap()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn stats(grid_node_count: u32) -> Stats {
        Stats {
            grid_node_count: Some(grid_node_count),
            compute: Some(ComputeStats {
                remaining_time_sec: 2.5,
                last_frame_time_sec: 0.5,
                last_frame_substeps: 7,
            }),
        }
    }

    #[test]
    fn append_writes_header_once() {
        let dir = temp_dir("append");
        append_stats(&dir, 0, &Stats::default()).unwrap();
        append_stats(&dir, 1, &stats(42)).unwrap();
        assert_eq!(
            rows(&dir),
            [STATS_HEADER.trim_end(), "0,,,,", "1,42,0.5,2.5,7"]
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn truncate_cuts_suffix() {
        let dir = temp_dir("truncate");
        for frame in 0..5 {
            append_stats(&dir, frame, &stats(frame as u32)).unwrap();
        }
        truncate_stats(&dir, 3).unwrap();
        assert_eq!(rows(&dir).len(), 1 + 3);
        assert!(rows(&dir)[3].starts_with("2,"));

        truncate_stats(&dir, 0).unwrap();
        assert_eq!(rows(&dir), [STATS_HEADER.trim_end()]);

        append_stats(&dir, 0, &stats(1)).unwrap();
        assert_eq!(rows(&dir).len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn truncate_drops_partial_row() {
        let dir = temp_dir("partial");
        append_stats(&dir, 0, &stats(0)).unwrap();
        std::fs::OpenOptions::new()
            .append(true)
            .open(stats_path(&dir))
            .unwrap()
            .write_all(b"1,3")
            .unwrap();
        truncate_stats(&dir, 10).unwrap();
        assert_eq!(rows(&dir).len(), 2);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn truncate_missing_file() {
        let dir = temp_dir("missing");
        truncate_stats(&dir, 0).unwrap();
        assert!(!stats_path(&dir).exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
