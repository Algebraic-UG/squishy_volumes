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
        Arc, Mutex, PoisonError, Weak,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(thiserror::Error, Debug)]
#[error("the computation was canceled")]
pub struct Canceled;

#[derive(thiserror::Error, Debug)]
pub enum HarnessError {
    #[error("can't create new report scope for '{new}', already reporting on '{old}'")]
    AlreadyHasSubReport { old: String, new: String },
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

    pub fn check(&self) -> Result<(), Canceled> {
        if self.is_canceled() {
            Err(Canceled)
        } else {
            Ok(())
        }
    }

    pub fn is_canceled(&self) -> bool {
        !self.run.load(Ordering::Relaxed)
    }

    pub fn step_to(&self, completed_steps: usize) -> Result<(), Canceled> {
        self.check()?;
        self.report
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .info
            .completed_steps = completed_steps;
        Ok(())
    }

    pub fn step(&self) -> Result<(), Canceled> {
        self.check()?;
        self.report
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .info
            .completed_steps += 1;
        Ok(())
    }

    pub fn scope(
        &self,
        label: String,
        steps_to_completion: NonZero<usize>,
    ) -> Result<Self, HarnessError> {
        let mut current_report = self.report.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(existing) = current_report.sub_report.upgrade() {
            let old = existing
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
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

    pub fn get_infos(&self) -> Vec<ReportInfo> {
        let mut infos: Vec<ReportInfo> = Default::default();

        let mut report = self.report.clone();
        loop {
            let lock = report.lock().unwrap_or_else(PoisonError::into_inner);
            infos.push(lock.info.clone());
            let Some(sub_report) = lock.sub_report.upgrade() else {
                break;
            };
            drop(lock);
            report = sub_report;
        }

        infos
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn poisoned_report_still_returns_infos() {
        let harness = Harness::new("outer".to_string(), NonZero::new(3).unwrap());
        let sub_harness = harness
            .scope("inner".to_string(), NonZero::new(2).unwrap())
            .unwrap();
        sub_harness.step().unwrap();

        let poisoner = {
            let harness = harness.clone();
            let sub_harness = sub_harness.clone();
            std::thread::spawn(move || {
                let _outer = harness.report.lock().unwrap();
                let _inner = sub_harness.report.lock().unwrap();
                panic!("harness test panic");
            })
        };
        assert!(poisoner.join().is_err());
        assert!(harness.report.is_poisoned());
        assert!(sub_harness.report.is_poisoned());

        harness.step().unwrap();
        let infos = harness.get_infos();

        let progress = infos
            .iter()
            .map(|info| (info.label.as_str(), info.completed_steps))
            .collect::<Vec<_>>();
        assert_eq!(progress, [("outer", 1), ("inner", 1)]);
    }
}
