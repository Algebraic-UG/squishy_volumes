// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    num::NonZero,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(thiserror::Error, Debug)]
pub enum HarnessError {
    #[error("The operation has been canceled.")]
    Canceled,
    #[error("Can't create new report scope for '{new}', already reporting on '{old}'")]
    AlreadyHasSubReport { old: String, new: String },
    #[error("Something went really wrong and the report mutex is poisoned")]
    ReportMutexPoisoned,
}

#[derive(Clone, serde::Serialize)]
pub struct ReportInfo {
    pub label: String,
    pub completed_steps: usize,
    pub steps_to_completion: NonZero<usize>,
}

struct Report {
    info: ReportInfo,
    sub_report: Weak<Mutex<Report>>,
}

impl Report {
    fn new(label: String, steps_to_completion: NonZero<usize>) -> Self {
        Self {
            info: ReportInfo {
                label,
                completed_steps: 0,
                steps_to_completion,
            },
            sub_report: Weak::new(),
        }
    }
}

#[derive(Clone)]
pub struct Harness {
    run: Arc<AtomicBool>,
    report: Arc<Mutex<Report>>,
}

impl Harness {
    pub fn new(label: String, steps_to_completion: NonZero<usize>) -> Self {
        let run = Arc::new(AtomicBool::new(true));
        let report = Arc::new(Mutex::new(Report::new(label, steps_to_completion)));
        Self { run, report }
    }

    pub fn cancel(&self) {
        self.run.store(false, Ordering::Relaxed);
    }

    pub fn check(&self) -> Result<(), HarnessError> {
        if self.is_canceled() {
            Err(HarnessError::Canceled)
        } else {
            Ok(())
        }
    }

    pub fn is_canceled(&self) -> bool {
        !self.run.load(Ordering::Relaxed)
    }

    pub fn step_to(&self, completed_steps: usize) -> Result<(), HarnessError> {
        self.report
            .lock()
            .map_err(|_| HarnessError::ReportMutexPoisoned)?
            .info
            .completed_steps = completed_steps;
        Ok(())
    }

    pub fn step(&self) -> Result<(), HarnessError> {
        self.report
            .lock()
            .map_err(|_| HarnessError::ReportMutexPoisoned)?
            .info
            .completed_steps += 1;
        Ok(())
    }

    pub fn scope(
        &self,
        label: String,
        steps_to_completion: NonZero<usize>,
    ) -> Result<Self, HarnessError> {
        let mut current_report = self
            .report
            .lock()
            .map_err(|_| HarnessError::ReportMutexPoisoned)?;
        if let Some(existing) = current_report.sub_report.upgrade() {
            let old = existing
                .lock()
                .map_err(|_| HarnessError::ReportMutexPoisoned)?
                .info
                .label
                .clone();
            return Err(HarnessError::AlreadyHasSubReport { old, new: label });
        }

        let run = self.run.clone();
        let report = Arc::new(Mutex::new(Report::new(label, steps_to_completion)));

        current_report.sub_report = Arc::downgrade(&report);

        Ok(Self { run, report })
    }

    pub fn get_infos(&self) -> Result<Vec<ReportInfo>, HarnessError> {
        let mut infos: Vec<ReportInfo> = Default::default();

        let mut report = self.report.clone();
        loop {
            let lock = report
                .lock()
                .map_err(|_| HarnessError::ReportMutexPoisoned)?;
            infos.push(lock.info.clone());
            let Some(sub_report) = lock.sub_report.upgrade() else {
                break;
            };
            drop(lock);
            report = sub_report;
        }

        Ok(infos)
    }
}
