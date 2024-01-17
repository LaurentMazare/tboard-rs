use pyo3::prelude::*;

use ::tboard as tb;
use tb::Error;

#[allow(unused)]
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

#[pyclass]
struct EventWriter {
    inner: tb::EventWriter<std::fs::File>,
}

#[pymethods]
impl EventWriter {
    #[new]
    fn new(path: std::path::PathBuf) -> PyResult<Self> {
        let inner = tb::EventWriter::create(path).map_err(w)?;
        Ok(Self { inner })
    }

    #[pyo3(signature = (tag, scalar_value, global_step=0))]
    fn add_scalar(&mut self, tag: &str, scalar_value: f32, global_step: i64) -> PyResult<()> {
        self.inner.write_scalar(global_step, tag, scalar_value).map_err(w)
    }

    fn flush(&mut self) -> PyResult<()> {
        self.inner.flush().map_err(w)
    }
}

#[pymodule]
fn tboard(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<EventWriter>()?;
    Ok(())
}
