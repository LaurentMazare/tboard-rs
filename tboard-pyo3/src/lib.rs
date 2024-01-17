use pyo3::prelude::*;

use ::tboard as tb;
use tb::Error;

#[allow(unused)]
fn w_py(err: PyErr) -> Error {
    Error::msg(err)
}

#[allow(unused)]
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

#[derive(Copy, Clone, PartialEq, Eq)]
enum OnError {
    Log,
    Raise,
}

#[pyclass]
struct EventWriter {
    inner: tb::EventWriter<std::fs::File>,
    on_error: OnError,
    logdir: String,
}

impl EventWriter {
    fn handle_err(&self, r: tb::Result<()>) -> PyResult<()> {
        match self.on_error {
            OnError::Raise => r.map_err(w),
            OnError::Log => {
                if let Err(err) = r {
                    eprintln!("error logging to {:?}: {err:?}", self.inner.filename());
                }
                Ok(())
            }
        }
    }
}

#[pymethods]
impl EventWriter {
    #[new]
    #[pyo3(signature = (logdir, on_error="raise"))]
    fn new(logdir: String, on_error: &str) -> PyResult<Self> {
        let inner = tb::EventWriter::create(&logdir).map_err(w)?;
        let on_error = match on_error {
            "raise" => OnError::Raise,
            "log" => OnError::Log,
            on_error => py_bail!("on_error can only be 'raise' or 'log', got '{on_error}'"),
        };
        Ok(Self { inner, logdir, on_error })
    }

    #[pyo3(signature = (tag, scalar_value, global_step=0))]
    fn add_scalar(&mut self, tag: &str, scalar_value: f32, global_step: i64) -> PyResult<()> {
        let res = self.inner.write_scalar(global_step, tag, scalar_value);
        self.handle_err(res)?;
        self.flush()
    }

    fn flush(&mut self) -> PyResult<()> {
        let res = self.inner.flush();
        self.handle_err(res)
    }

    #[getter]
    fn logdir(&self) -> &str {
        &self.logdir
    }

    #[getter]
    fn filename(&self) -> Option<&str> {
        self.inner.filename().and_then(|v| v.to_str())
    }
}

#[pymodule]
fn tboard(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<EventWriter>()?;
    Ok(())
}
