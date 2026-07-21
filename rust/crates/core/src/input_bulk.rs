// SPDX-License-Identifier: MIT
//
// Copyright 2025  Algebraic UG (haftungsbeschränkt)
//
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE_MIT file or at
// https://opensource.org/licenses/MIT.

use squishy_volumes_api::InputBulk;

#[derive(Debug, thiserror::Error)]
pub enum InputBulkError {
    #[error("Expected data to be bools")]
    ExpectedBool,
    #[error("Expected data to be floats")]
    ExpectedFloat,
    #[error("Expected data to be ints")]
    ExpectedInt,

    #[error("The flags had a different size before")]
    FlagsLengthChanged,

    #[error("Failed to cast sclice: {0}")]
    CastSlice(#[from] bytemuck::PodCastError),

    #[error("Input Error: {0}")]
    InputError(#[from] squishy_volumes_file_input::InputError),
}

pub trait InputBulkExt {
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    fn len(&self) -> usize;
    fn as_bools(&self) -> Result<&[bool], InputBulkError>;
    fn as_floats(&self) -> Result<&[f32], InputBulkError>;
    fn as_ints(&self) -> Result<&[i32], InputBulkError>;
}

impl InputBulkExt for InputBulk<'_> {
    fn len(&self) -> usize {
        match self {
            InputBulk::Bool(slice) => slice.len(),
            InputBulk::Floats(slice) => slice.len(),
            InputBulk::Ints(slice) => slice.len(),
        }
    }

    fn as_bools(&self) -> Result<&[bool], InputBulkError> {
        if let InputBulk::Bool(slice) = self {
            Ok(slice)
        } else {
            Err(InputBulkError::ExpectedBool)
        }
    }

    fn as_floats(&self) -> Result<&[f32], InputBulkError> {
        if let InputBulk::Floats(slice) = self {
            Ok(slice)
        } else {
            Err(InputBulkError::ExpectedFloat)
        }
    }

    fn as_ints(&self) -> Result<&[i32], InputBulkError> {
        if let InputBulk::Ints(slice) = self {
            Ok(slice)
        } else {
            Err(InputBulkError::ExpectedInt)
        }
    }
}
