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
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc,
};

use squishy_volumes_file_frame::Frame;

use super::*;

pub const QUEUED_FRAMES: usize = 2;

type StoreJob = (Frame, Stats);
type StoreResult = Result<(), CacheWritingError>;

struct SenderAndThread {
    sender: mpsc::SyncSender<StoreJob>,
    thread: std::thread::JoinHandle<StoreResult>,
}

pub struct StoreThread {
    sender_and_thread: Option<SenderAndThread>,
    stop_requested: Arc<AtomicBool>,
}

pub struct FrameSender(mpsc::SyncSender<StoreJob>);

#[allow(clippy::result_large_err)]
impl FrameSender {
    pub fn send(&self, frame: Frame, stats: Stats) -> StoreResult {
        match self.0.try_send((frame, stats)) {
            Ok(()) => Ok(()),
            Err(mpsc::TrySendError::Full(job)) => {
                tracing::warn!(
                    "store queue is full ({QUEUED_FRAMES} frames), computation waits for the disk"
                );
                self.0.send(job).map_err(|_| CacheWritingError::Sending)
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(CacheWritingError::Sending),
        }
    }
}

#[allow(clippy::result_large_err)]
impl StoreThread {
    pub fn new(
        cache_dir: std::path::PathBuf,
        total_bytes_on_disk: Arc<AtomicU64>,
        available_frames: Arc<AtomicUsize>,
    ) -> Self {
        Self::spawn(move |frame, stats| {
            write_frame(
                &cache_dir,
                &total_bytes_on_disk,
                &available_frames,
                frame,
                &stats,
            )
        })
    }

    fn spawn(write: impl FnMut(Frame, Stats) -> StoreResult + Send + 'static) -> Self {
        tracing::info!("starting store thread");
        let (sender, receiver) = mpsc::sync_channel::<StoreJob>(QUEUED_FRAMES);
        let stop_requested = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop_requested = stop_requested.clone();
            std::thread::spawn(move || store_until_stopped(receiver, stop_requested, write))
        };
        Self {
            sender_and_thread: Some(SenderAndThread { sender, thread }),
            stop_requested,
        }
    }

    pub fn sender(&self) -> Result<FrameSender, CacheWritingError> {
        Ok(FrameSender(
            self.sender_and_thread
                .as_ref()
                .ok_or(CacheWritingError::ThreadGone)?
                .sender
                .clone(),
        ))
    }

    pub fn check(&mut self) -> StoreResult {
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

    pub fn stop(&mut self) {
        if let Some(thread) = self.request_stop() {
            join_and_log(thread);
        }
    }

    fn request_stop(&mut self) -> Option<std::thread::JoinHandle<StoreResult>> {
        self.stop_requested.store(true, Ordering::Release);
        let SenderAndThread { sender, thread } = self.sender_and_thread.take()?;
        drop(sender);
        Some(thread)
    }
}

impl Drop for StoreThread {
    fn drop(&mut self) {
        self.stop();
    }
}

#[allow(clippy::result_large_err)]
fn store_until_stopped(
    receiver: mpsc::Receiver<StoreJob>,
    stop_requested: Arc<AtomicBool>,
    mut write: impl FnMut(Frame, Stats) -> StoreResult,
) -> StoreResult {
    while let Ok((frame, stats)) = receiver.recv() {
        if stop_requested.load(Ordering::Acquire) {
            tracing::info!("discarding queued frames");
            break;
        }
        write(frame, stats)?;
    }
    tracing::info!("terminating store thread");
    Ok(())
}

#[allow(clippy::result_large_err)]
fn write_frame(
    cache_dir: &std::path::Path,
    total_bytes_on_disk: &AtomicU64,
    available_frames: &AtomicUsize,
    frame: Frame,
    stats: &Stats,
) -> StoreResult {
    let frame_index = available_frames.load(Ordering::Relaxed);
    total_bytes_on_disk.fetch_add(
        frame.write(frame_path(cache_dir, frame_index))?,
        Ordering::Relaxed,
    );
    available_frames.fetch_add(1, Ordering::Relaxed);
    // after the frame, so every row has a frame
    append_stats(cache_dir, frame_index, stats).map_err(CacheWritingError::WriteStats)?;
    tracing::debug!("stored frame {frame_index}");
    Ok(())
}

fn join_and_log(thread: std::thread::JoinHandle<StoreResult>) {
    match thread.join() {
        Err(e) => {
            tracing::error!("store thread paniced: {e:?}");
        }
        Ok(Err(e)) => {
            tracing::error!("store thread error: {e}");
        }
        Ok(Ok(())) => {}
    }
}

#[cfg(test)]
#[allow(clippy::result_large_err)]
mod tests {
    use std::time::Duration;

    use super::*;

    const HANG_TIMEOUT: Duration = Duration::from_secs(10);

    struct GatedStore {
        store_thread: StoreThread,
        written: Arc<AtomicUsize>,
        started_writing: mpsc::Receiver<()>,
        release_writes: mpsc::Sender<()>,
    }

    fn gated_store() -> GatedStore {
        let written = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_writing) = mpsc::channel();
        let (release_writes, release_rx) = mpsc::channel::<()>();
        let store_thread = {
            let written = written.clone();
            StoreThread::spawn(move |_, _| {
                started_tx.send(()).ok();
                release_rx.recv().ok();
                written.fetch_add(1, Ordering::Relaxed);
                Ok(())
            })
        };
        GatedStore {
            store_thread,
            written,
            started_writing,
            release_writes,
        }
    }

    fn within_timeout<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || tx.send(f()).ok());
        rx.recv_timeout(HANG_TIMEOUT).expect("operation hung")
    }

    fn fill_while_first_frame_is_written(gated: &GatedStore) -> FrameSender {
        let sender = gated.store_thread.sender().unwrap();
        sender.send(Frame::default(), Stats::default()).unwrap();
        gated.started_writing.recv_timeout(HANG_TIMEOUT).unwrap();
        for _ in 0..QUEUED_FRAMES {
            sender.send(Frame::default(), Stats::default()).unwrap();
        }
        sender
    }

    #[test]
    fn stop_finishes_current_frame_and_discards_queue() {
        let mut gated = gated_store();
        let sender = fill_while_first_frame_is_written(&gated);
        let blocked_producer = {
            let sender = gated.store_thread.sender().unwrap();
            std::thread::spawn(move || sender.send(Frame::default(), Stats::default()))
        };

        let thread = gated.store_thread.request_stop().unwrap();
        drop(gated.release_writes);
        within_timeout(move || join_and_log(thread));

        let _accepted_or_rejected = within_timeout(move || blocked_producer.join().unwrap());
        assert_eq!(gated.written.load(Ordering::Relaxed), 1);
        assert!(matches!(
            sender.send(Frame::default(), Stats::default()),
            Err(CacheWritingError::Sending)
        ));
    }

    #[test]
    fn write_error_is_reported_by_check() {
        let mut store_thread = StoreThread::spawn(|_, _| {
            Err(CacheWritingError::WriteStats(std::io::Error::other(
                "disk full",
            )))
        });
        let sender = store_thread.sender().unwrap();
        sender.send(Frame::default(), Stats::default()).unwrap();

        let error = within_timeout(move || {
            loop {
                match store_thread.check() {
                    Ok(()) => std::thread::yield_now(),
                    Err(error) => return error,
                }
            }
        });

        assert!(matches!(error, CacheWritingError::WriteStats(_)));
        assert!(matches!(
            sender.send(Frame::default(), Stats::default()),
            Err(CacheWritingError::Sending)
        ));
    }
}
