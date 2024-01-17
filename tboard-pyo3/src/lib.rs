#![allow(unused)]
use pyo3::prelude::*;
use std::collections::HashMap;
use std::sync::Arc;

use ::tboard as tb;
use tb::{bail, Error, Result};

fn w_py(err: PyErr) -> Error {
    Error::msg(err)
}

fn w<E: std::error::Error>(err: E) -> PyErr {
    pyo3::exceptions::PyValueError::new_err(err.to_string())
}

#[macro_export]
macro_rules! py_bail {
    ($msg:literal $(,)?) => {
        return Err(pyo3::exceptions::PyValueError::new_err(format!($msg)))
    };
    ($err:expr $(,)?) => {
        return Err(pyo3::exceptions::PyValueError::new_err(format!($err)))
    };
    ($fmt:expr, $($arg:tt)*) => {
        return Err(pyo3::exceptions::PyValueError::new_err(format!($fmt, $($arg)*)))
    };
}

#[pymodule]
fn tboard(_py: Python, m: &PyModule) -> PyResult<()> {
    Ok(())
}
