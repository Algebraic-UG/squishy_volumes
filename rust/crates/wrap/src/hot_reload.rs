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
use std::{any::Any, panic::catch_unwind, sync::Mutex, thread::spawn, time::Duration};

use crate::BUG;

#[cfg_attr(
    debug_assertions,
    hot_lib_reloader::hot_module(
        dylib = "squishy_volumes_hot",
        lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug"),
        loaded_lib_name_template = "{lib_name}-hot-{pid}-{load_counter}"
    )
)]
#[cfg_attr(
    not(debug_assertions),
    hot_lib_reloader::hot_module(
        dylib = "squishy_volumes_hot",
        lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/release"),
        loaded_lib_name_template = "{lib_name}-hot-{pid}-{load_counter}"
    )
)]
pub mod squishy_volumes_hot_reload {
    hot_functions_from_file!("crates/hot/src/lib.rs");

    #[allow(unused)]
    pub use squishy_volumes_hot::*;

    #[lib_change_subscription]
    pub fn subscribe() -> hot_lib_reloader::LibReloadObserver {}

    pub fn api_build_stamp() -> Option<u64> {
        let lib_loader = __lib_loader();
        let lib_loader = lib_loader.read().ok()?;
        let build_stamp = unsafe {
            lib_loader
                .get_symbol::<*const u64>(b"SQUISHY_VOLUMES_API_BUILD_STAMP\0")
                .ok()?
        };
        Some(unsafe { **build_stamp })
    }
}

pub fn check_api_build_stamp(expected: u64, found: Option<u64>) -> Result<(), String> {
    match found {
        Some(found) if found == expected => Ok(()),
        found => Err(format!(
            "The Squishy Volumes library doesn't match the extension \
(extension built {expected}, library built {}).
Rebuild the extension (scripts/rebuild_api.sh) and restart Blender.",
            found.map_or("unknown".to_string(), |found| found.to_string())
        )),
    }
}

pub enum HotState {
    Uninitialized,
    Ready(Box<dyn squishy_volumes_api::Context>),
    Unavailable(String),
}

impl HotState {
    pub fn context(&mut self) -> Result<&mut dyn squishy_volumes_api::Context> {
        match self {
            Self::Uninitialized => bail!(BUG),
            Self::Ready(context) => Ok(context.as_mut()),
            Self::Unavailable(reason) => bail!("{reason}"),
        }
    }
}

lazy_static! {
    static ref STATE: Mutex<HotState> = Mutex::new(HotState::Uninitialized);
}

const RELOAD_TIMEOUT: Duration = Duration::from_secs(10);

const RELOAD_TIMED_OUT: &str = "Reloading the Squishy Volumes library timed out.
Rebuild the extension (scripts/rebuild_api.sh) and restart Blender.";

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}

fn load_state() -> HotState {
    match catch_unwind(|| {
        check_api_build_stamp(
            squishy_volumes_api::BUILD_STAMP,
            squishy_volumes_hot_reload::api_build_stamp(),
        )
        .map(|()| squishy_volumes_hot_reload::create_context())
    }) {
        Ok(Ok(context)) => HotState::Ready(context),
        Ok(Err(reason)) => HotState::Unavailable(reason),
        Err(payload) => HotState::Unavailable(format!(
            "Loading the Squishy Volumes library failed: {}",
            panic_message(payload.as_ref())
        )),
    }
}

pub fn initialize() {
    let mut state = STATE.lock().unwrap();
    *state = load_state();
}

pub fn core_build_info() -> Result<BuildInfo> {
    try_with_context(|_| Ok(squishy_volumes_hot_reload::build_info()))
}

pub fn try_with_context<R, F: FnOnce(&mut dyn squishy_volumes_api::Context) -> Result<R>>(
    f: F,
) -> Result<R> {
    let Ok(mut state) = STATE.lock() else {
        bail!(BUG)
    };
    f(state.context()?)
}

pub fn handle_reload() {
    let _ = spawn(|| {
        let lib_observer = squishy_volumes_hot_reload::subscribe();
        loop {
            let update_blocker = lib_observer.wait_for_about_to_reload();

            // held across the reload, so no call reaches a half-loaded library
            let mut state = STATE.lock().unwrap();

            // the context must be dropped while the old library is still loaded
            *state = HotState::Uninitialized;
            drop(update_blocker);
            *state = if lib_observer.wait_for_reload_timeout(RELOAD_TIMEOUT) {
                load_state()
            } else {
                HotState::Unavailable(RELOAD_TIMED_OUT.to_string())
            };
        }
    });
}
