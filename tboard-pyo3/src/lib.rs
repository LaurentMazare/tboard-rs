use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};

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
struct EventIter {
    reader: tb::SummaryReader<std::fs::File>,
}

#[pymethods]
impl EventIter {
    fn __iter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> {
        slf
    }

    fn __next__(mut slf: PyRefMut<'_, Self>, py: Python) -> PyResult<Option<PyObject>> {
        use ::tboard::tensorboard::event::What;
        use ::tboard::tensorboard::summary::value::Value::SimpleValue;
        use std::ops::DerefMut;

        let slf = slf.deref_mut();
        match slf.reader.next() {
            None => Ok(None),
            Some(ok_or_err) => {
                let event = ok_or_err.map_err(w)?;
                let dict = PyDict::new(py);
                dict.set_item("wall_time", event.wall_time)?;
                dict.set_item("step", event.step)?;
                dict.set_item("source_metadata", event.source_metadata.map(|v| v.writer))?;
                let mut values = vec![];
                if let Some(what) = event.what {
                    match what {
                        What::Summary(summary) => {
                            dict.set_item("kind", "summary")?;
                            for value in summary.value.iter() {
                                let v = PyDict::new(py);
                                v.set_item("tag", &value.tag)?;
                                v.set_item("node_name", &value.node_name)?;
                                match value.value {
                                    Some(SimpleValue(s)) => v.set_item("value", s)?,
                                    Some(_) => {}
                                    None => v.set_item("value", None::<usize>)?,
                                }
                                // v.set_item("metadata", value.metadata);
                                values.push(v)
                            }
                        }
                        What::MetaGraphDef(def) => {
                            dict.set_item("kind", "meta_graph_def")?;
                            dict.set_item("meta_graph_def", PyBytes::new(py, &def))?;
                        }
                        What::TaggedRunMetadata(trm) => {
                            dict.set_item("kind", "tagged_run_metadata")?;
                            dict.set_item("tag", trm.tag)?;
                            dict.set_item("run_metadata", trm.run_metadata)?;
                        }
                        What::FileVersion(version) => {
                            dict.set_item("kind", "file_version")?;
                            dict.set_item("file_version", version)?;
                        }
                        What::SessionLog(sl) => {
                            dict.set_item("kind", "session_log")?;
                            dict.set_item("status", sl.status)?;
                            dict.set_item("msg", sl.msg)?;
                            dict.set_item("checkpoint_path", sl.checkpoint_path)?;
                        }
                        What::LogMessage(lm) => {
                            dict.set_item("kind", "log_message")?;
                            dict.set_item("level", lm.level)?;
                            dict.set_item("message", lm.message)?;
                        }
                        What::GraphDef(gd) => {
                            dict.set_item("kind", "graph_def")?;
                            dict.set_item("graph_def", PyBytes::new(py, &gd))?;
                        }
                    }
                }
                let what = PyList::new(py, values.iter());
                dict.set_item("what", what)?;
                Ok(Some(dict.into()))
            }
        }
    }
}

#[pyclass]
struct EventReader {
    filename: std::path::PathBuf,
}

#[pymethods]
impl EventReader {
    #[new]
    #[pyo3(signature = (filename,))]
    fn new(filename: &str) -> PyResult<Self> {
        let filename = std::path::PathBuf::from(filename);
        if !filename.is_file() {
            py_bail!("{filename:?} is not a file")
        }
        Ok(Self { filename })
    }

    fn __iter__(slf: PyRef<'_, Self>) -> PyResult<EventIter> {
        let reader = std::fs::File::open(&slf.filename)?;
        let reader = tb::SummaryReader::new(reader);
        Ok(EventIter { reader })
    }
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

    #[pyo3(signature = (tag, pcm_data, sample_rate, global_step=0))]
    fn add_audio(
        &mut self,
        tag: &str,
        pcm_data: Vec<f32>,
        sample_rate: u32,
        global_step: i64,
    ) -> PyResult<()> {
        let res = self.inner.write_pcm_as_wav(global_step, tag, &pcm_data, sample_rate);
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
    m.add_class::<EventReader>()?;
    m.add_class::<EventWriter>()?;
    Ok(())
}
