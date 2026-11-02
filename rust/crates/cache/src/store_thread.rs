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

use std::sync::{
    Arc,
    atomic::{AtomicU64, AtomicUsize, Ordering},
    mpsc,
};

use super::*;

struct SenderAndThread {
    sender: mpsc::Sender<(squishy_volumes_file_frame::Frame, Stats)>,
    thread: std::thread::JoinHandle<Result<(), CacheWritingError>>,
}

pub struct StoreThread {
    sender_and_thread: Option<SenderAndThread>,
}

#[allow(clippy::result_large_err)]
impl StoreThread {
    pub fn new(
        cache_dir: std::path::PathBuf,
        total_bytes_on_disk: Arc<AtomicU64>,
        available_frames: Arc<AtomicUsize>,
    ) -> Self {
        tracing::info!("starting store thread");
        // TODO: this should be bounded?
        let (store_tx, store_rx) = mpsc::channel::<(squishy_volumes_file_frame::Frame, Stats)>();
        let thread = std::thread::spawn(move || -> Result<(), CacheWritingError> {
            while let Ok((frame, stats)) = store_rx.recv() {
                let frame_index = available_frames.load(Ordering::Relaxed);
                total_bytes_on_disk.fetch_add(
                    frame.write(frame_path(&cache_dir, frame_index))?,
                    Ordering::Relaxed,
                );
                available_frames.fetch_add(1, Ordering::Relaxed);
                // after the frame, so every row has a frame
                append_stats(&cache_dir, frame_index, &stats)
                    .map_err(CacheWritingError::WriteStats)?;
                tracing::debug!("stored frame {frame_index}");
            }
            tracing::info!("terminating store thread");
            Ok(())
        });
        Self {
            sender_and_thread: Some(SenderAndThread {
                sender: store_tx,
                thread,
            }),
        }
    }

    pub fn store(
        &self,
        frame: squishy_volumes_file_frame::Frame,
        stats: Stats,
    ) -> Result<(), CacheWritingError> {
        self.sender_and_thread
            .as_ref()
            .ok_or(CacheWritingError::ThreadGone)?
            .sender
            .send((frame, stats))
            .map_err(|_| CacheWritingError::Sending)
    }

    pub fn check(&mut self) -> Result<(), CacheWritingError> {
        if !self
            .sender_and_thread
            .as_ref()
            .ok_or(CacheWritingError::ThreadGone)?
            .thread
            .is_finished()
        {
            return Ok(());
        }

        self.sender_and_thread
            .take()
            .ok_or(CacheWritingError::ThreadGone)?
            .thread
            .join()
            .map_err(|_| CacheWritingError::StoreThreadPaniced)??;
        Err(CacheWritingError::ThreadStopped)
    }
}

impl Drop for StoreThread {
    fn drop(&mut self) {
        let Some(SenderAndThread { sender, thread }) = self.sender_and_thread.take() else {
            return;
        };
        drop(sender);
        match thread.join() {
            Err(e) => {
                tracing::error!("store thread paniced: {e:?}");
            }
            Ok(Err(e)) => {
                tracing::error!("store thread error: {e}");
            }
            _ => {}
        }
    }
}
