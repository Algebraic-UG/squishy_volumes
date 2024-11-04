// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use std::array::from_fn;

use anyhow::{Context, Error, Result, ensure};
use base64::prelude::*;
use blended_mpm_api::T;
use nalgebra::{Quaternion, Vector3};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct SerializedVector {
    pub dtype: String,
    pub data: String,
}

impl TryFrom<SerializedVector> for Vec<Vector3<T>> {
    type Error = Error;

    fn try_from(value: SerializedVector) -> Result<Self> {
        ensure!(value.dtype == "float32");
        Ok(BASE64_STANDARD
            .decode(value.data)?
            .as_slice()
            .chunks_exact(12)
            .map(|chunk| {
                let bytes: [u8; 12] = chunk.try_into().unwrap();
                from_fn(|i| {
                    f32::from_le_bytes([
                        bytes[4 * i],
                        bytes[4 * i + 1],
                        bytes[4 * i + 2],
                        bytes[4 * i + 3],
                    ]) as T
                })
                .into()
            })
            .collect())
    }
}

impl TryFrom<SerializedVector> for Vec<Quaternion<T>> {
    type Error = Error;

    fn try_from(value: SerializedVector) -> Result<Self> {
        ensure!(value.dtype == "float32");
        Ok(BASE64_STANDARD
            .decode(value.data)?
            .as_slice()
            .chunks_exact(16)
            .map(|chunk| {
                let bytes: [u8; 16] = chunk.try_into().unwrap();
                from_fn(|i| {
                    f32::from_le_bytes([
                        bytes[4 * i],
                        bytes[4 * i + 1],
                        bytes[4 * i + 2],
                        bytes[4 * i + 3],
                    ]) as T
                })
                .into()
            })
            .collect())
    }
}

impl TryFrom<SerializedVector> for Vec<Option<Vector3<T>>> {
    type Error = Error;

    fn try_from(value: SerializedVector) -> Result<Self> {
        ensure!(value.dtype == "float32");
        Ok(BASE64_STANDARD
            .decode(value.data)?
            .as_slice()
            .chunks_exact(12)
            .map(|chunk| {
                let bytes: [u8; 12] = chunk.try_into().unwrap();
                let vector = from_fn(|i| {
                    f32::from_le_bytes([
                        bytes[4 * i],
                        bytes[4 * i + 1],
                        bytes[4 * i + 2],
                        bytes[4 * i + 3],
                    ]) as T
                })
                .into();
                (vector != Vector3::zeros()).then_some(vector)
            })
            .collect())
    }
}

impl TryFrom<SerializedVector> for Vec<[u32; 3]> {
    type Error = Error;

    fn try_from(value: SerializedVector) -> Result<Self> {
        ensure!(value.dtype == "int32");
        BASE64_STANDARD
            .decode(value.data)?
            .as_slice()
            .chunks_exact(12)
            .map(|chunk| {
                let bytes: [u8; 12] = chunk.try_into().unwrap();
                let [a, b, c]: [i32; 3] = from_fn(|i| {
                    i32::from_le_bytes([
                        bytes[4 * i],
                        bytes[4 * i + 1],
                        bytes[4 * i + 2],
                        bytes[4 * i + 3],
                    ])
                });
                Ok([a.try_into()?, b.try_into()?, c.try_into()?])
            })
            .collect::<Result<_>>()
            .context("negative index")
    }
}
