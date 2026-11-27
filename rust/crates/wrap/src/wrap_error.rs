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

use pyo3::{exceptions::PyRuntimeError, prelude::*};
use squishy_volumes_api::CodedError;

#[pyclass(extends = PyRuntimeError, frozen, module = "squishy_volumes_wrap")]
pub struct CoreError {
    #[pyo3(get)]
    error_code: Option<String>,
}

pub struct WrapError(anyhow::Error);

pub type Result<T> = std::result::Result<T, WrapError>;

impl<E: Into<anyhow::Error>> From<E> for WrapError {
    fn from(error: E) -> Self {
        Self(error.into())
    }
}

impl From<WrapError> for PyErr {
    fn from(WrapError(error): WrapError) -> Self {
        if error.source().is_none() && error.is::<PyErr>() {
            return error.into();
        }
        let code = error
            .downcast_ref::<CodedError>()
            .map(|coded| coded.code.clone());
        let message = format!("{error:?}");
        Python::attach(|py| {
            new_core_error(py, message, code).unwrap_or_else(|creation_failed| creation_failed)
        })
    }
}

fn new_core_error(py: Python<'_>, message: String, error_code: Option<String>) -> PyResult<PyErr> {
    let core_error = Bound::new(py, CoreError { error_code })?;
    core_error.setattr("args", (message,))?;
    Ok(PyErr::from_value(core_error.into_any()))
}
