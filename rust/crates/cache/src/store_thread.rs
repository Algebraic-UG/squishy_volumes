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
    sender: mpsc::Sender<squishy_volumes_file_frame::IoState>,
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
        let (store_tx, store_rx) = mpsc::channel::<squishy_volumes_file_frame::IoState>();
        let thread = std::thread::spawn(move || -> Result<(), CacheWritingError> {
            while let Ok(state) = store_rx.recv() {
                total_bytes_on_disk.fetch_add(
                    state.write(frame_path(
                        &cache_dir,
                        available_frames.load(Ordering::Relaxed),
                    ))?,
                    Ordering::Relaxed,
                );
                available_frames.fetch_add(1, Ordering::Relaxed);
                tracing::debug!(
                    "stored frame {}",
                    available_frames.load(Ordering::Relaxed) - 1
                );
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
        state: squishy_volumes_file_frame::IoState,
    ) -> Result<(), CacheWritingError> {
        self.sender_and_thread
            .as_ref()
            .ok_or(CacheWritingError::ThreadGone)?
            .sender
            .send(state)
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
