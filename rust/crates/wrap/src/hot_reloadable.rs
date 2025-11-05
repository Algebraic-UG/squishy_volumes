// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use anyhow::{bail, Context, Result};
use build_info::BuildInfo;
use lazy_static::lazy_static;
use std::sync::Mutex;

#[cfg(feature = "hot_reload")]
use std::thread::spawn;

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

lazy_static! {
    static ref LOCK: Mutex<Option<Box<dyn squishy_volumes_api::Context>>> =
        Mutex::new(Default::default());
}

#[derive(serde::Serialize)]
pub struct CombinedBuildInfo {
    pub wrapper: BuildInfo,
    pub core_libs: BuildInfo,
}

build_info::build_info!(fn _build_info);
impl CombinedBuildInfo {
    pub fn new() -> Self {
        Self {
            wrapper: _build_info().clone(),
            core_libs: squishy_volumes_hot_reload::build_info(),
        }
    }
}

const BUG: &str = "You encountered a bug in Squishy Volumes.
It's unlikely that the process can recover.
Please consider restarting the application.";

pub fn initialize() {
    if let Ok(mut guard) = LOCK.lock() {
        *guard = Some(squishy_volumes_hot_reload::create_context());
    } else {
        eprintln!("{BUG}");
    }
}

pub fn try_with_context<R, F: FnOnce(&mut dyn squishy_volumes_api::Context) -> Result<R>>(
    f: F,
) -> Result<R> {
    if let Ok(mut guard) = LOCK.lock() {
        return f(guard.as_mut().context(BUG)?.as_mut());
    }
    bail!(BUG)
}

pub fn with_context<R, F: FnOnce(&mut dyn squishy_volumes_api::Context) -> R>(f: F) -> Result<R> {
    try_with_context(|c| Ok(f(c)))
}

#[cfg(feature = "hot_reload")]
pub fn handle_reload() {
    let _ = spawn(|| {
        let lib_observer = squishy_volumes_hot_reload::subscribe();
        loop {
            // wait for reload and block it
            let update_blocker = lib_observer.wait_for_about_to_reload();

            // wait for any library calls to finish and block further calls
            let mut context_guard = LOCK.lock().unwrap();

            // cleanup
            let context = context_guard.take();

            // this should block until any threads are joined and any resources are dropped
            drop(context);

            // let the library update commence and wait until it's finished
            drop(update_blocker);
            lib_observer.wait_for_reload();

            // fresh context with new lib version
            *context_guard = Some(squishy_volumes_hot_reload::create_context());

            // context_guard is dropped and calls can continue
        }
    });
}
