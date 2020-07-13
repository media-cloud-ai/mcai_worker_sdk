#[macro_use]
extern crate serde_derive;

use std::{env, fs};

use pyo3::{prelude::*, types::*};

use crate::helpers::get_destination_paths;
use crate::parameters::{build_parameters, PythonWorkerParameters};
use mcai_worker_sdk::{
  job::*,
  publish_job_progression, start_worker,
  worker::{Parameter, ParameterType},
  McaiChannel, MessageError, MessageEvent, Result, Version,
};

mod helpers;
mod parameters;

#[derive(Debug, Clone)]
struct PythonWorkerEvent {}

impl PythonWorkerEvent {
  fn read_python_file() -> String {
    let filename = env::var("PYTHON_WORKER_FILENAME").unwrap_or_else(|_| "worker.py".to_string());

    fs::read_to_string(&filename)
      .unwrap_or_else(|_| panic!("unable to open and read file: {}", filename))
  }

  fn get_string_from_module(&self, method: &str) -> String {
    let contents = PythonWorkerEvent::read_python_file();

    let gil = Python::acquire_gil();
    let py = gil.python();
    let python_module = PyModule::from_code(py, &contents, "worker.py", "worker")
      .expect("unable to create the python module");

    let response: String = python_module
      .call0(method)
      .unwrap_or_else(|_| panic!("unable to call {} in your module", method))
      .extract()
      .unwrap_or_else(|_| panic!("unable to found a return value for {} function", method));

    response
  }

  fn get_parameters() -> Vec<Parameter> {
    let contents = PythonWorkerEvent::read_python_file();

    let gil = Python::acquire_gil();
    let py = gil.python();
    let python_module = PyModule::from_code(py, &contents, "worker.py", "worker")
      .expect("unable to create the python module");

    let response = python_module
      .call0("get_parameters")
      .unwrap_or_else(|_| panic!("unable to call get_parameters in your module".to_string()))
      .downcast::<PyList>()
      .unwrap();

    let mut parameters = vec![];

    for item in response.iter() {
      let object = item.downcast::<PyDict>().expect("not a python dict");

      let label = object
        .get_item("label")
        .expect("missing label in parameter")
        .to_string();
      let identifier = object
        .get_item("identifier")
        .expect("missing identifier in parameter")
        .to_string();

      let kind_list = object
        .get_item("kind")
        .expect("missing kind in parameter")
        .downcast::<PyList>()
        .unwrap();

      let mut parameter_types = vec![];

      for kind in kind_list.iter() {
        let value = kind
          .downcast::<PyString>()
          .expect("not a python string")
          .to_string()
          .unwrap();
        let parameter_type: ParameterType = serde_json::from_str(&format!("{:?}", value)).unwrap();
        parameter_types.push(parameter_type);
      }

      let required = object
        .get_item("required")
        .unwrap_or_else(|| PyBool::new(py, false).as_ref())
        .is_true()
        .unwrap();

      parameters.push(Parameter {
        label,
        identifier,
        kind: parameter_types,
        required,
      });
    }

    parameters
  }
}

#[pyclass]
struct CallbackHandle {
  channel: Option<McaiChannel>,
  job_id: u64,
}

#[pymethods]
impl CallbackHandle {
  fn publish_job_progression(&self, value: u8) -> bool {
    publish_job_progression(self.channel.clone(), self.job_id, value).is_ok()
  }
}

impl MessageEvent<PythonWorkerParameters> for PythonWorkerEvent {
  fn get_name(&self) -> String {
    self.get_string_from_module("get_name")
  }

  fn get_short_description(&self) -> String {
    self.get_string_from_module("get_short_description")
  }

  fn get_description(&self) -> String {
    self.get_string_from_module("get_description")
  }

  fn get_version(&self) -> Version {
    Version::parse(&self.get_string_from_module("get_version"))
      .expect("unable to parse version (please use SemVer format)")
  }

  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: PythonWorkerParameters,
    mut job_result: JobResult,
  ) -> Result<JobResult> {
    let contents = PythonWorkerEvent::read_python_file();

    let gil = Python::acquire_gil();
    let py = gil.python();
    let traceback = py.import("traceback").unwrap();
    let python_module = PyModule::from_code(py, &contents, "worker.py", "worker")
      .expect("unable to create the python module");

    let list_of_parameters = build_parameters(parameters, py)?;

    let callback_handle = CallbackHandle {
      channel,
      job_id: job_result.get_job_id(),
    };

    match python_module.call1("process", (callback_handle, list_of_parameters)) {
      Ok(response) => {
        if let Some(mut destination_paths) = get_destination_paths(response) {
          job_result = job_result.with_destination_paths(&mut destination_paths);
        }

        Ok(job_result.with_status(JobStatus::Completed))
      }
      Err(error) => {
        let stacktrace = if let Some(tb) = &error.ptraceback {
          let locals = [("traceback", traceback)].into_py_dict(py);

          locals.set_item("tb", tb).unwrap();

          py.eval("traceback.format_tb(tb)", None, Some(locals))
            .expect("Unknown python error, unable to get the stacktrace")
            .to_string()
        } else {
          "Unknown python error, no stackstrace".to_string()
        };

        let locals = [("error", error)].into_py_dict(py);

        let error_msg = py
          .eval("repr(error)", None, Some(locals))
          .expect("Unknown python error, unable to get the error message")
          .to_string();

        let error_message = format!("{}\n\nStacktrace:\n{}", error_msg, stacktrace);

        let result = job_result
          .with_status(JobStatus::Error)
          .with_message(&error_message);
        Err(MessageError::ProcessingError(result))
      }
    }
  }
}

static PYTHON_WORKER_EVENT: PythonWorkerEvent = PythonWorkerEvent {};

fn main() {
  start_worker(PYTHON_WORKER_EVENT.clone());
}
