// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

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
