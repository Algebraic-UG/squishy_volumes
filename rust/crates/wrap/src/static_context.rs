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

use anyhow::{bail, Result};
use build_info::BuildInfo;
use lazy_static::lazy_static;
use std::sync::Mutex;

use crate::BUG;

lazy_static! {
    static ref CONTEXT: Mutex<Box<dyn squishy_volumes_api::Context>> =
        Mutex::new(squishy_volumes_hot::create_context());
}

pub fn initialize() {
    lazy_static::initialize(&CONTEXT);
}

pub fn try_with_context<R, F: FnOnce(&mut dyn squishy_volumes_api::Context) -> Result<R>>(
    f: F,
) -> Result<R> {
    let Ok(mut context) = CONTEXT.lock() else {
        bail!(BUG)
    };
    f(context.as_mut())
}

pub fn core_build_info() -> Result<BuildInfo> {
    Ok(squishy_volumes_hot::build_info())
}
