// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{Context, Result, bail};
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

use crate::State;

use super::frame_path;

pub struct StoreThread {
    store_tx: Sender<State>,
    thread: Option<JoinHandle<Result<()>>>,
}

impl StoreThread {
    pub fn new(
        cache_dir: PathBuf,
        bytes_on_disk: Arc<AtomicU64>,
        available_frames: Arc<AtomicUsize>,
    ) -> Self {
        info!("starting store thread");
        let temp_file_path = cache_dir.join("temp.bin");
        let (store_tx, store_rx) = channel();
        let thread = Some(spawn(move || -> Result<()> {
            while let Ok(state) = store_rx.recv() {
                let mut file = File::create(&temp_file_path).context("temp file creation")?;
                file.write_all(&serialize(&state).context("state serialization")?)
                    .context("frame file writing")?;
                bytes_on_disk.fetch_add(
                    file.metadata().context("frame size")?.len(),
                    Ordering::Relaxed,
                );
                rename(
                    &temp_file_path,
                    frame_path(&cache_dir, available_frames.load(Ordering::Relaxed)),
                )
                .context("frame file renaming")?;
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

    pub fn store(&self, state: State) -> Result<()> {
        Ok(self.store_tx.send(state)?)
    }

    pub fn check(&mut self) -> Result<()> {
        if self
            .thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
        {
            return Ok(());
        }
        self.thread
            .take()
            .context("store thread handle is gone")?
            .join()
            .unwrap()?;
        bail!("store thread shutdown unexpectedly");
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
