// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    num::NonZero,
    sync::{Arc, Mutex, Weak},
};

use squishy_volumes_api::Task;

pub const REPORT_STRIDE: usize = 1 << 12;

// TODO: this should also include the run flag
#[derive(Clone)]
pub enum Report {
    Stack(Arc<Mutex<ReportDetail>>),
    Store(Weak<Mutex<ReportDetail>>),
}

pub struct ReportDetail {
    info: ReportInfo,
    sub_reports: Vec<Weak<Mutex<ReportDetail>>>,
}

#[derive(Clone)]
pub struct ReportInfo {
    pub name: String,
    pub completed_steps: usize,
    pub steps_to_completion: NonZero<usize>,
}

impl From<Report> for Option<Task> {
    fn from(report: Report) -> Self {
        let report = (match report {
            Report::Stack(arc) => Some(arc),
            Report::Store(weak) => weak.upgrade(),
        })?;
        let lock = report.lock().unwrap();
        let ReportInfo {
            name,
            completed_steps,
            steps_to_completion,
        } = lock.info.clone();
        Some(Task {
            name,
            completed_steps,
            steps_to_completion: steps_to_completion.get(),
            sub_tasks: lock
                .sub_reports
                .iter()
                .cloned()
                .map(Report::Store)
                .filter_map(Into::into)
                .collect(),
        })
    }
}

impl Report {
    pub fn new(info: ReportInfo) -> Self {
        Self::Stack(Arc::new(Mutex::new(ReportDetail {
            info,
            sub_reports: Default::default(),
        })))
    }

    pub fn as_store(&self) -> Self {
        match self {
            Self::Stack(arc) => Self::Store(Arc::downgrade(arc)),
            Self::Store(weak) => Self::Store(weak.clone()),
        }
    }

    pub fn new_sub(&self, info: ReportInfo) -> Self {
        let Self::Stack(report) = self else {
            panic!("trying to create a new sub report from store");
        };
        let sub = Arc::new(Mutex::new(ReportDetail {
            info,
            sub_reports: Default::default(),
        }));
        report
            .lock()
            .unwrap()
            .sub_reports
            .push(Arc::downgrade(&sub));
        Self::Stack(sub)
    }

    pub fn step(&self) {
        let Self::Stack(report) = self else {
            panic!("trying to step a report from store");
        };
        report.lock().unwrap().info.completed_steps += 1;
    }

    pub fn set_completed(&self, completed_steps: usize) {
        let Self::Stack(report) = self else {
            panic!("trying to set completed steps of a report from store");
        };
        report.lock().unwrap().info.completed_steps = completed_steps;
    }
}
