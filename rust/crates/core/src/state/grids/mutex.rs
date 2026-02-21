// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::{
    ops::{Deref, DerefMut},
    sync::MutexGuard,
};

use serde::{Deserialize, Serialize};

#[derive(Default, Serialize, Deserialize)]
pub struct Mutex<T: ?Sized>(std::sync::Mutex<T>);
//pub struct Mutex<T: ?Sized>(spin::Mutex<T>);
impl<T> Deref for Mutex<T> {
    type Target = std::sync::Mutex<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> DerefMut for Mutex<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: Clone> Clone for Mutex<T> {
    fn clone(&self) -> Self {
        Self(self.0.lock().unwrap().deref().clone().into())
    }
}

impl<T> Mutex<T> {
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0.lock().unwrap()
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(value: T) -> Self {
        Self(value.into())
    }
}
