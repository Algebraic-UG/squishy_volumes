// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use bincode::serialize;
use std::{
    fs::{File, rename},
    io::Write,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
        mpsc::{Sender, channel},
    },
    thread::{JoinHandle, spawn},
};
use tracing::{debug, info};

use crate::{cache::CacheWritingError, state::State};

use super::frame_path;

pub struct StoreThread {
    store_tx: Sender<State>,
    thread: Option<JoinHandle<Result<(), CacheWritingError>>>,
}

impl StoreThread {
    pub fn new(
        cache_dir: PathBuf,
        total_bytes_on_disk: Arc<AtomicU64>,
        available_frames: Arc<AtomicUsize>,
    ) -> Self {
        info!("starting store thread");
        let temp_file_path = cache_dir.join("temp.bin");
        let (store_tx, store_rx) = channel();
        let thread = Some(spawn(move || -> Result<(), CacheWritingError> {
            while let Ok(state) = store_rx.recv() {
                let mut file =
                    File::create(&temp_file_path).map_err(CacheWritingError::CreateFrame)?;
                file.write_all(&serialize(&state)?)
                    .map_err(CacheWritingError::WriteFrame)?;
                total_bytes_on_disk.fetch_add(file.metadata()?.len(), Ordering::Relaxed);
                rename(
                    &temp_file_path,
                    frame_path(&cache_dir, available_frames.load(Ordering::Relaxed)),
                )
                .map_err(CacheWritingError::MoveFrame)?;
                available_frames.fetch_add(1, Ordering::Relaxed);
                debug!(
                    "stored frame {}",
                    available_frames.load(Ordering::Relaxed) - 1
                );
            }
            Ok(())
        }));
        Self { store_tx, thread }
    }

    pub fn store(&self, state: State) -> Result<(), CacheWritingError> {
        Ok(self.store_tx.send(state)?)
    }

    pub fn check(&mut self) -> Result<(), CacheWritingError> {
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return Ok(());
        }
        self.thread
            .take()
            .ok_or(CacheWritingError::ThreadGone)?
            .join()
            .unwrap()?;
        Err(CacheWritingError::ThreadStopped)
    }
}

impl Drop for StoreThread {
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.store_tx = channel().0;
        let _ = thread.join().unwrap();
    }
}
