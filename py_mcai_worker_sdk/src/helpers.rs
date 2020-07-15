#[cfg(feature = "media")]
use mcai_worker_sdk::{MessageError, Result};
use pyo3::{prelude::*, types::*};

#[cfg(not(feature = "media"))]
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
              let mut destination_paths: Vec<String> = vec![];

              for path in path_list.iter() {
                if let Ok(value) = path.downcast::<PyString>() {
                  if let Ok(string_value) = value.to_string() {
                    destination_paths.push(string_value.to_string());
                  }
                }
              }
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
      let mut items = vec![];
      for item in py_list.iter() {
        if let Ok(value) = item.downcast::<PyLong>() {
          if let Ok(int_value) = value.extract::<usize>() {
            items.push(int_value);
          }
        }
      }
      items
    })
    .map_err(|e| {
      MessageError::RuntimeError(format!(
        "unable to access init_process(..) python response: {:?}",
        e
      ))
    })
}
