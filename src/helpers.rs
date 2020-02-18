use pyo3::{prelude::*, types::*};

pub fn get_destination_paths(response: &PyAny) -> Option<Vec<String>> {
  if response.is_none() {
    return None;
  }

  response
    .downcast_ref::<PyDict>()
    .map(|object| {
      object
        .get_item("destination_paths")
        .map(|response_paths| {
          response_paths
            .downcast_ref::<PyList>()
            .map(|path_list| {
              let mut destination_paths: Vec<String> = vec![];

              for path in path_list.iter() {
                if let Ok(value) = path.downcast_ref::<PyString>() {
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
