use pyo3::{prelude::*, types::*};

#[cfg(feature = "media")]
use mcai_worker_sdk::{MessageError, Result, StreamDescriptor};

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
pub fn get_stream_indexes(response: &PyAny) -> Result<Vec<StreamDescriptor>> {
  response
    .downcast::<PyList>()
    .map(|py_list| {
      py_list
        .iter()
        .map(|value| value.extract::<StreamDescriptor>())
        .filter(|extracted| extracted.is_ok())
        .map(|extracted| extracted.unwrap())
        .collect()
    })
    .map_err(|e| {
      MessageError::RuntimeError(format!(
        "unable to access init_process(..) python response: {:?}",
        e
      ))
    })
}

#[test]
pub fn test_py_err_to_string() {
  let error_message = "Error message";
  let gil = Python::acquire_gil();
  let py = gil.python();

  PyErr::new::<pyo3::exceptions::TypeError, _>(error_message.clone()).restore(py);
  let py_err = PyErr::fetch(py);

  let expected_message = format!("TypeError(\'{}\'", error_message);
  assert!(py_err_to_string(py, py_err).contains(&expected_message));
}

#[test]
pub fn test_get_destination_paths() {
  let destination_paths = vec![
    "/path/to/destination/file_1".to_string(),
    "/path/to/destination/file_2".to_string(),
    "/path/to/destination/file_3".to_string(),
  ];
  let gil = Python::acquire_gil();
  let py = gil.python();

  let py_list = PyList::new(py, destination_paths.clone());
  let py_dict = PyDict::new(py);
  let result = py_dict.set_item("destination_paths", py_list);
  assert!(result.is_ok());

  let py_any: &PyAny = py_dict.into();

  let result = get_destination_paths(py_any);
  assert!(result.is_some());
  assert_eq!(destination_paths, result.unwrap());
}

#[test]
pub fn test_get_destination_paths_without_key() {
  let gil = Python::acquire_gil();
  let py = gil.python();

  let py_dict = PyDict::new(py);

  let py_any: &PyAny = py_dict.into();

  let result = get_destination_paths(py_any);
  assert!(result.is_none());
}

#[test]
pub fn test_get_destination_paths_without_list_value() {
  let gil = Python::acquire_gil();
  let py = gil.python();

  let py_dict = PyDict::new(py);
  let result = py_dict.set_item("destination_paths", "some_value");
  assert!(result.is_ok());

  let py_any: &PyAny = py_dict.into();

  let result = get_destination_paths(py_any);
  assert!(result.is_none());
}

#[test]
#[cfg(feature = "media")]
pub fn test_get_stream_indexes() {
  let stream_indexes = vec![StreamDescriptor::new_audio(0, vec![], vec![], vec![])];
  let gil = Python::acquire_gil();
  let py = gil.python();

  let py_list: PyObject = stream_indexes.into_py(py);
  let py_any: &PyAny = py_list.cast_as(py).unwrap();

  let result = get_stream_indexes(&py_any);
  assert!(result.is_ok());
  let result = result.unwrap();
  assert_eq!(1, result.len());
}

#[test]
#[cfg(feature = "media")]
pub fn test_get_stream_indexes_without_list() {
  let gil = Python::acquire_gil();
  let py = gil.python();

  let py_string = PyString::new(py, "this_is_not_a_list!");
  let py_any: &PyAny = py_string.into();

  let expected_error = MessageError::RuntimeError(
    "unable to access init_process(..) python response: PyDowncastError".to_string(),
  );

  let result = get_stream_indexes(py_any);
  assert!(result.is_err());
  assert_eq!(expected_error, result.unwrap_err());
}
