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
use std::{
    any::Any,
    panic::catch_unwind,
    sync::{Mutex, MutexGuard, PoisonError},
};

#[cfg(feature = "hot_reload")]
use std::{thread::spawn, time::Duration};

#[cfg(not(feature = "hot_reload"))]
pub use squishy_volumes_hot as squishy_volumes_hot_reload;

#[cfg(feature = "hot_reload")]
#[cfg_attr(
    debug_assertions,
    hot_lib_reloader::hot_module(
        dylib = "squishy_volumes_hot",
        lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/debug")
    )
)]
#[cfg_attr(
    not(debug_assertions),
    hot_lib_reloader::hot_module(
        dylib = "squishy_volumes_hot",
        lib_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/release")
    )
)]
pub mod squishy_volumes_hot_reload {
    hot_functions_from_file!("crates/hot/src/lib.rs");

    #[allow(unused)]
    pub use squishy_volumes_hot::*;

    #[lib_change_subscription]
    pub fn subscribe() -> hot_lib_reloader::LibReloadObserver {}
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

#[derive(serde::Serialize)]
pub struct CombinedBuildInfo {
    pub wrapper: BuildInfo,
    pub core_libs: BuildInfo,
}

build_info::build_info!(fn _build_info);
impl CombinedBuildInfo {
    pub fn new() -> Result<Self> {
        Ok(Self {
            wrapper: _build_info().clone(),
            core_libs: try_with_context(|_| Ok(squishy_volumes_hot_reload::build_info()))?,
        })
    }
}

const BUG: &str = "You encountered a bug in Squishy Volumes.
It's unlikely that the process can recover.
Please consider restarting the application.";

#[cfg(feature = "hot_reload")]
const RELOAD_TIMEOUT: Duration = Duration::from_secs(10);

#[cfg(feature = "hot_reload")]
const RELOAD_TIMED_OUT: &str = "Reloading the Squishy Volumes library timed out.
Rebuild the extension (scripts/rebuild_api.sh) and restart Blender.";

fn lock_state() -> MutexGuard<'static, HotState> {
    STATE.lock().unwrap_or_else(PoisonError::into_inner)
}

fn panic_message(payload: &(dyn Any + Send)) -> &str {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
}

fn load_state() -> HotState {
    match catch_unwind(squishy_volumes_hot_reload::create_context) {
        Ok(context) => HotState::Ready(context),
        Err(payload) => HotState::Unavailable(format!(
            "Loading the Squishy Volumes library failed: {}",
            panic_message(payload.as_ref())
        )),
    }
}

pub fn initialize() {
    let mut state = lock_state();
    *state = load_state();
    drop(state);
    STATE.clear_poison();
}

pub fn try_with_context<R, F: FnOnce(&mut dyn squishy_volumes_api::Context) -> Result<R>>(
    f: F,
) -> Result<R> {
    let Ok(mut state) = STATE.lock() else {
        bail!(BUG)
    };
    f(state.context()?)
}

#[cfg(feature = "hot_reload")]
pub fn handle_reload() {
    let _ = spawn(|| {
        let lib_observer = squishy_volumes_hot_reload::subscribe();
        loop {
            let update_blocker = lib_observer.wait_for_about_to_reload();

            // held across the reload, so no call reaches a half-loaded library
            let mut state = lock_state();

            // the context must be dropped while the old library is still loaded
            *state = HotState::Uninitialized;
            drop(update_blocker);
            *state = if lib_observer.wait_for_reload_timeout(RELOAD_TIMEOUT) {
                load_state()
            } else {
                HotState::Unavailable(RELOAD_TIMED_OUT.to_string())
            };

            drop(state);
            STATE.clear_poison();
        }
    });
}
