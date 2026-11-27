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
    Arc, Mutex, PoisonError,
    atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
    mpsc,
};

use squishy_volumes_file_frame::Frame;
use squishy_volumes_util::panic_payload_to_string;

use super::*;

pub const QUEUED_FRAMES: usize = 2;
const STOP_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

type StoreJob = (Frame, Stats);
type StoreResult = Result<(), WriteError>;

#[derive(Clone, Default)]
struct StoreFailure(Arc<Mutex<Option<WriteError>>>);

impl StoreFailure {
    fn set(&self, error: WriteError) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some(error);
    }

    fn take(&self) -> Option<WriteError> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).take()
    }
}

pub struct StoreThread {
    thread: Option<std::thread::JoinHandle<()>>,
    stop_requested: Arc<AtomicBool>,
    failure: StoreFailure,
}

#[derive(Clone)]
pub struct FrameSender {
    sender: mpsc::SyncSender<StoreJob>,
    failure: StoreFailure,
}

impl FrameSender {
    pub fn send(&self, frame: Frame, stats: Stats) -> StoreResult {
        match self.sender.try_send((frame, stats)) {
            Ok(()) => Ok(()),
            Err(mpsc::TrySendError::Full(job)) => {
                tracing::warn!(
                    "store queue is full ({QUEUED_FRAMES} frames), computation waits for the disk"
                );
                self.sender.send(job).map_err(|_| self.failure_or_sending())
            }
            Err(mpsc::TrySendError::Disconnected(_)) => Err(self.failure_or_sending()),
        }
    }

    fn failure_or_sending(&self) -> WriteError {
        self.failure.take().unwrap_or(WriteError::Sending)
    }
}

impl StoreThread {
    pub fn idle() -> Self {
        Self {
            thread: None,
            stop_requested: Arc::default(),
            failure: StoreFailure::default(),
        }
    }

    pub fn start(
        &mut self,
        cache_dir: std::path::PathBuf,
        space_budget: SpaceBudget,
        available_frames: Arc<AtomicUsize>,
    ) -> FrameSender {
        let total_bytes_on_disk = space_budget.total_bytes_on_disk.clone();
        self.start_with(space_budget, move |frame, stats| {
            write_frame(
                &cache_dir,
                &total_bytes_on_disk,
                &available_frames,
                frame,
                &stats,
            )
        })
    }

    fn start_with(
        &mut self,
        space_budget: SpaceBudget,
        write: impl FnMut(Frame, Stats) -> StoreResult + Send + 'static,
    ) -> FrameSender {
        self.stop();
        tracing::info!("starting store thread");
        let (sender, receiver) = mpsc::sync_channel::<StoreJob>(QUEUED_FRAMES);
        self.stop_requested = Arc::default();
        self.failure = StoreFailure::default();
        let stop_requested = self.stop_requested.clone();
        let failure = self.failure.clone();
        self.thread = Some(std::thread::spawn(move || {
            // the failure must be set before the receiver drops, senders look for it once disconnected
            if let Err(error) = store_until_stopped(&receiver, stop_requested, &space_budget, write)
            {
                failure.set(error);
            }
        }));
        FrameSender {
            sender,
            failure: self.failure.clone(),
        }
    }

    pub fn running(&self) -> bool {
        let finished = self
            .thread
            .as_ref()
            .is_none_or(std::thread::JoinHandle::is_finished);
        if finished {
            // pairs with the thread's exit, so its last frame counter updates are visible
            std::sync::atomic::fence(Ordering::Acquire);
        }
        !finished
    }

    pub fn check(&mut self) -> StoreResult {
        if !self
            .thread
            .as_ref()
            .is_some_and(std::thread::JoinHandle::is_finished)
        {
            return Ok(());
        }
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|payload| WriteError::StoreThreadPanicked {
                    message: panic_payload_to_string(payload),
                })?;
        }
        self.failure.take().map_or(Ok(()), Err)
    }

    pub fn stop(&mut self) {
        self.request_stop();
        if let Some(thread) = self.thread.take() {
            join_and_log(thread);
        }
        if let Some(error) = self.failure.take() {
            tracing::error!("store thread error: {error}");
        }
    }

    fn request_stop(&self) {
        self.stop_requested.store(true, Ordering::Release);
    }
}

impl Drop for StoreThread {
    fn drop(&mut self) {
        self.stop();
    }
}

fn store_until_stopped(
    receiver: &mpsc::Receiver<StoreJob>,
    stop_requested: Arc<AtomicBool>,
    space_budget: &SpaceBudget,
    mut write: impl FnMut(Frame, Stats) -> StoreResult,
) -> StoreResult {
    loop {
        let job = receiver.recv_timeout(STOP_POLL_INTERVAL);
        if stop_requested.load(Ordering::Acquire) {
            tracing::info!("discarding queued frames");
            break;
        }
        match job {
            Ok((frame, stats)) => {
                space_budget.ensure_room_for_a_frame()?;
                write(frame, stats)?
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }
    tracing::info!("terminating store thread");
    Ok(())
}

fn write_frame(
    cache_dir: &std::path::Path,
    total_bytes_on_disk: &AtomicU64,
    available_frames: &AtomicUsize,
    frame: Frame,
    stats: &Stats,
) -> StoreResult {
    let frame_index = available_frames.load(Ordering::Relaxed);
    total_bytes_on_disk.fetch_add(
        frame
            .write(frame_path(cache_dir, frame_index))
            .map_err(Box::new)?,
        Ordering::Relaxed,
    );
    available_frames.fetch_add(1, Ordering::Release);
    // after the frame, so every row has a frame
    append_stats(cache_dir, frame_index, stats)?;
    tracing::debug!("stored frame {frame_index}");
    Ok(())
}

fn join_and_log(thread: std::thread::JoinHandle<()>) {
    if let Err(payload) = thread.join() {
        tracing::error!(
            payload = panic_payload_to_string(payload),
            "store thread panicked"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    const HANG_TIMEOUT: Duration = Duration::from_secs(10);
    const FRAME_BYTES: u64 = 100;

    struct GatedStore {
        store_thread: StoreThread,
        sender: FrameSender,
        written: Arc<AtomicUsize>,
        started_writing: mpsc::Receiver<()>,
        release_writes: mpsc::Sender<()>,
    }

    fn budget(max_bytes_on_disk: u64) -> SpaceBudget {
        SpaceBudget::new(
            std::path::PathBuf::new(),
            Arc::default(),
            SpaceLimits {
                max_bytes_on_disk,
                check_free_space: false,
            },
            available_space_on_disk,
        )
    }

    fn unlimited_budget() -> SpaceBudget {
        budget(u64::MAX)
    }

    fn gated_store(space_budget: SpaceBudget) -> GatedStore {
        let written = Arc::new(AtomicUsize::new(0));
        let (started_tx, started_writing) = mpsc::channel();
        let (release_writes, release_rx) = mpsc::channel::<()>();
        let mut store_thread = StoreThread::idle();
        let sender = {
            let written = written.clone();
            let total_bytes_on_disk = space_budget.total_bytes_on_disk.clone();
            store_thread.start_with(space_budget, move |_, _| {
                started_tx.send(()).ok();
                release_rx.recv().ok();
                written.fetch_add(1, Ordering::Relaxed);
                total_bytes_on_disk.fetch_add(FRAME_BYTES, Ordering::Relaxed);
                Ok(())
            })
        };
        GatedStore {
            store_thread,
            sender,
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

    fn fill_while_first_frame_is_written(gated: &GatedStore) {
        gated
            .sender
            .send(Frame::default(), Stats::default())
            .unwrap();
        gated.started_writing.recv_timeout(HANG_TIMEOUT).unwrap();
        for _ in 0..QUEUED_FRAMES {
            gated
                .sender
                .send(Frame::default(), Stats::default())
                .unwrap();
        }
    }

    fn wait_until_stopped(store_thread: StoreThread) -> StoreThread {
        within_timeout(move || {
            while store_thread.running() {
                std::thread::yield_now();
            }
            store_thread
        })
    }

    #[test]
    fn stop_finishes_current_frame_and_discards_queue() {
        let gated = gated_store(unlimited_budget());
        fill_while_first_frame_is_written(&gated);
        let blocked_producer = {
            let sender = gated.sender.clone();
            std::thread::spawn(move || sender.send(Frame::default(), Stats::default()))
        };

        let GatedStore {
            mut store_thread,
            sender,
            written,
            release_writes,
            ..
        } = gated;
        store_thread.request_stop();
        drop(release_writes);
        within_timeout(move || store_thread.stop());

        let _accepted_or_rejected = within_timeout(move || blocked_producer.join().unwrap());
        assert_eq!(written.load(Ordering::Relaxed), 1);
        assert!(matches!(
            sender.send(Frame::default(), Stats::default()),
            Err(WriteError::Sending)
        ));
    }

    #[test]
    fn stop_does_not_wait_for_live_senders() {
        let mut store_thread = StoreThread::idle();
        let _sender = store_thread.start_with(unlimited_budget(), |_, _| Ok(()));

        within_timeout(move || store_thread.stop());
    }

    #[test]
    fn running_until_queued_frames_are_written_after_last_sender_is_dropped() {
        let gated = gated_store(unlimited_budget());
        fill_while_first_frame_is_written(&gated);
        let GatedStore {
            store_thread,
            sender,
            written,
            release_writes,
            ..
        } = gated;

        drop(sender);
        assert!(store_thread.running());
        for _ in 0..=QUEUED_FRAMES {
            release_writes.send(()).unwrap();
        }
        let mut store_thread = wait_until_stopped(store_thread);

        assert_eq!(written.load(Ordering::Relaxed), QUEUED_FRAMES + 1);
        assert!(store_thread.check().is_ok());
    }

    #[test]
    fn reaching_the_limit_discards_the_queued_frames_and_reports_it() {
        let gated = gated_store(budget(FRAME_BYTES / 2));
        fill_while_first_frame_is_written(&gated);
        let GatedStore {
            store_thread,
            sender,
            written,
            release_writes,
            ..
        } = gated;

        drop(sender);
        for _ in 0..=QUEUED_FRAMES {
            release_writes.send(()).ok();
        }
        let mut store_thread = wait_until_stopped(store_thread);

        assert_eq!(written.load(Ordering::Relaxed), 1);
        assert!(matches!(
            store_thread.check(),
            Err(WriteError::ExceedingSpace)
        ));
    }

    #[test]
    fn sending_after_the_limit_stopped_the_thread_reports_the_limit() {
        let gated = gated_store(budget(FRAME_BYTES / 2));
        fill_while_first_frame_is_written(&gated);
        let blocked_producer = {
            let sender = gated.sender.clone();
            std::thread::spawn(move || sender.send(Frame::default(), Stats::default()))
        };
        let GatedStore {
            store_thread,
            sender,
            release_writes,
            ..
        } = gated;

        for _ in 0..=QUEUED_FRAMES {
            release_writes.send(()).ok();
        }
        let _stopped = wait_until_stopped(store_thread);

        let blocked_result = within_timeout(move || blocked_producer.join().unwrap());
        let later_result = sender.send(Frame::default(), Stats::default());
        assert!(matches!(
            (blocked_result, later_result),
            (Err(WriteError::ExceedingSpace), Err(WriteError::Sending))
                | (Ok(()), Err(WriteError::ExceedingSpace))
        ));
    }

    #[test]
    fn running_while_a_sender_is_alive() {
        let mut store_thread = StoreThread::idle();
        let sender = store_thread.start_with(unlimited_budget(), |_, _| Ok(()));
        std::thread::sleep(STOP_POLL_INTERVAL * 2);
        assert!(store_thread.running());

        drop(sender);
        wait_until_stopped(store_thread);
    }

    #[test]
    fn write_error_is_reported_by_check_after_the_thread_ended() {
        let mut store_thread = StoreThread::idle();
        let sender = store_thread.start_with(unlimited_budget(), |_, _| {
            Err(WriteError::Stats {
                path: std::path::PathBuf::new(),
                source: std::io::Error::other("disk full"),
            })
        });
        sender.send(Frame::default(), Stats::default()).unwrap();

        let mut store_thread = wait_until_stopped(store_thread);

        assert!(matches!(
            store_thread.check(),
            Err(WriteError::Stats { .. })
        ));
        assert!(store_thread.check().is_ok());
        assert!(matches!(
            sender.send(Frame::default(), Stats::default()),
            Err(WriteError::Sending)
        ));
    }

    #[test]
    fn panic_is_reported_by_check_after_the_thread_ended() {
        let mut store_thread = StoreThread::idle();
        let sender =
            store_thread.start_with(unlimited_budget(), |_, _| panic!("store thread test panic"));
        sender.send(Frame::default(), Stats::default()).unwrap();

        let mut store_thread = wait_until_stopped(store_thread);

        assert!(matches!(
            store_thread.check(),
            Err(WriteError::StoreThreadPanicked { message })
                if message == "store thread test panic"
        ));
    }
}
