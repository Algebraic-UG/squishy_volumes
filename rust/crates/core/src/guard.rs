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
    ops::Deref,
    panic::{AssertUnwindSafe, catch_unwind},
    sync::OnceLock,
};

use squishy_volumes_util::panic_payload_to_string;

use crate::Error;

pub trait Breakable {
    fn panic_message(&self) -> &OnceLock<String>;
    fn broken(panic_message: String) -> Error;
}

#[allow(clippy::result_large_err)]
pub fn guard<R>(f: impl FnOnce() -> Result<R, Error>) -> Result<R, Error> {
    catch_panic(f).unwrap_or_else(|message| Err(caller_panic(message)))
}

#[allow(clippy::result_large_err)]
pub fn guard_breakable<B: Breakable, T: Deref<Target = B>, R>(
    mut breakable: T,
    f: impl FnOnce(&mut T) -> Result<R, Error>,
) -> Result<R, Error> {
    if let Some(message) = breakable.panic_message().get() {
        return Err(B::broken(message.clone()));
    }
    catch_panic(|| f(&mut breakable)).unwrap_or_else(|message| {
        let _ = breakable.panic_message().set(message.clone());
        Err(caller_panic(message))
    })
}

fn catch_panic<R>(f: impl FnOnce() -> R) -> Result<R, String> {
    // The hot-reloaded dylib has its own std, wrap can't catch its panics (#491)
    catch_unwind(AssertUnwindSafe(f)).map_err(panic_payload_to_string)
}

fn caller_panic(message: String) -> Error {
    Error::Panic {
        thread: "caller",
        message,
    }
}
