// This file is generated to alleviate https://github.com/Algebraic-UG/squishy_volumes/issues/83

use pyo3::prelude::*;

/// A Python module implemented in Rust. The name of this function must match
/// the `lib.name` setting in the `Cargo.toml`, else Python will not be able to
/// import the module.
#[pymodule]
pub fn squishy_volumes_wrap_0_1_21(m: &Bound<'_, PyModule>) -> PyResult<()> {
    super::squishy_volumes_wrap(m)
}
