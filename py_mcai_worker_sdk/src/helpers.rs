#[cfg(feature = "media")]
use mcai_worker_sdk::{MessageError, Result};
use pyo3::{prelude::*, types::*};

pub fn py_err_to_string(py: Python, error: PyErr) -> String {
  let locals = [("error", error)].into_py_dict(py);

  py.eval("repr(error)", None, Some(locals))
    .expect("Unknown python error, unable to get the error message")
    .to_string()
}

pub fn get_destination_paths(response: &PyAny) -> Option<Vec<String>> {
  if response.is_none() {
    return None;
  }

  response
    .downcast::<PyDict>()
    .map(|object| {
      object
        .get_item("destination_paths")
        .map(|response_paths| {
          response_paths
            .downcast::<PyList>()
            .map(|path_list| {
              let destination_paths = path_list
                .iter()
                .map(|item| item.downcast::<PyString>())
                .filter(|downcast| downcast.is_ok())
                .map(|value| value.unwrap().to_string())
                .filter(|extract| extract.is_ok())
                .map(|string_value| string_value.unwrap().to_string())
                .collect();

              Some(destination_paths)
            })
            .unwrap_or(None)
        })
        .flatten()
    })
    .unwrap_or(None)
}

#[cfg(feature = "media")]
pub fn get_stream_indexes(response: &PyAny) -> Result<Vec<usize>> {
  response
    .downcast::<PyList>()
    .map(|py_list| {
      py_list
        .iter()
        .map(|item| item.downcast::<PyLong>())
        .filter(|downcast| downcast.is_ok())
        .map(|value| value.unwrap().extract::<usize>())
        .filter(|extract| extract.is_ok())
        .map(|int_value| int_value.unwrap())
        .collect()
    })
    .map_err(|e| {
      MessageError::RuntimeError(format!(
        "unable to access init_process(..) python response: {:?}",
        e
      ))
    })
}
