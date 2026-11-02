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
