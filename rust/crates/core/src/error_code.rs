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

use std::error::Error;

use squishy_volumes_api::CodedError;
use squishy_volumes_cache::WriteError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    InsufficientDiskSpace,
}

impl ErrorCode {
    fn name(self) -> &'static str {
        match self {
            Self::InsufficientDiskSpace => "insufficient_disk_space",
        }
    }

    fn of_cause(cause: &(dyn Error + 'static)) -> Option<Self> {
        match cause.downcast_ref::<WriteError>()? {
            WriteError::InsufficientDiskSpace { .. } => Some(Self::InsufficientDiskSpace),
            _ => None,
        }
    }

    pub fn of(error: &(dyn Error + 'static)) -> Option<Self> {
        std::iter::successors(Some(error), |&cause| cause.source()).find_map(Self::of_cause)
    }
}

pub fn with_error_code<E: Error + Send + Sync + 'static>(error: E) -> anyhow::Error {
    match ErrorCode::of(&error) {
        Some(code) => anyhow::Error::new(CodedError {
            code: code.name().into(),
            error: error.into(),
        }),
        None => error.into(),
    }
}

#[cfg(test)]
mod tests {
    use squishy_volumes_cache::Error as CacheError;

    use super::*;
    use crate::SimulationError;

    fn insufficient_disk_space() -> SimulationError {
        SimulationError::StoreFrame(CacheError::Write(WriteError::InsufficientDiskSpace {
            remaining: 2,
            available: 1,
        }))
    }

    fn exceeding_space() -> SimulationError {
        SimulationError::StoreFrame(CacheError::Write(WriteError::ExceedingSpace))
    }

    fn message_and_chain(error: &anyhow::Error) -> (String, String) {
        (format!("{error}"), format!("{error:?}"))
    }

    #[test]
    fn a_coded_error_carries_its_code_and_keeps_its_message_and_causes() {
        let plain = anyhow::Error::from(insufficient_disk_space());

        let coded = with_error_code(insufficient_disk_space());

        assert_eq!(
            coded
                .downcast_ref::<CodedError>()
                .map(|coded| coded.code.as_str()),
            Some("insufficient_disk_space")
        );
        assert_eq!(message_and_chain(&coded), message_and_chain(&plain));
    }

    #[test]
    fn an_error_without_a_code_is_unchanged() {
        let plain = anyhow::Error::from(exceeding_space());

        let uncoded = with_error_code(exceeding_space());

        assert!(uncoded.downcast_ref::<CodedError>().is_none());
        assert_eq!(message_and_chain(&uncoded), message_and_chain(&plain));
    }
}
