use crate::helpers::get_destination_paths;
use mcai_worker_sdk::{
  job::*,
  publish_job_progression, start_worker,
  worker::{Parameter, ParameterType},
  McaiChannel, MessageError, MessageEvent, ParameterValue, Version,
};
use pyo3::{prelude::*, types::*};
use std::{env, fs};

mod helpers;

#[derive(Debug)]
struct PythonWorkerEvent {}

impl PythonWorkerEvent {
  fn read_python_file(&self) -> String {
    let filename = env::var("PYTHON_WORKER_FILENAME").unwrap_or_else(|_| "worker.py".to_string());

    fs::read_to_string(&filename)
      .unwrap_or_else(|_| panic!("unable to open and read file: {}", filename))
  }

  fn get_string_from_module(&self, method: &str) -> String {
    let contents = self.read_python_file();

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
}

#[pyclass]
struct CallbackHandle {
  channel: Option<McaiChannel>,
  job: Job,
}

#[pymethods]
impl CallbackHandle {
  fn publish_job_progression(&self, value: u8) -> bool {
    publish_job_progression(self.channel.clone(), &self.job, value).is_ok()
  }
}

impl MessageEvent for PythonWorkerEvent {
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

  fn get_parameters(&self) -> Vec<Parameter> {
    let contents = self.read_python_file();

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

  fn process(
    &self,
    channel: Option<McaiChannel>,
    job: &Job,
    mut job_result: JobResult,
  ) -> Result<JobResult, MessageError> {
    let contents = self.read_python_file();

    let gil = Python::acquire_gil();
    let py = gil.python();
    let traceback = py.import("traceback").unwrap();
    let python_module = PyModule::from_code(py, &contents, "worker.py", "worker")
      .expect("unable to create the python module");

    let list_of_parameters = PyDict::new(py);
    if let Err(error) = self.build_parameters(job, py, list_of_parameters) {
      let result = job_result
        .with_status(JobStatus::Error)
        .with_message(&error);
      return Err(MessageError::ProcessingError(result));
    }

    let callback_handle = CallbackHandle {
      channel,
      job: job.clone(),
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

fn py_err_to_string(py: Python, error: PyErr) -> String {
  let locals = [("error", error)].into_py_dict(py);

  py.eval("repr(error)", None, Some(locals))
    .expect("Unknown python error, unable to get the error message")
    .to_string()
}

impl PythonWorkerEvent {
  fn build_parameters(
    &self,
    job: &Job,
    py: Python,
    list_of_parameters: &PyDict,
  ) -> Result<(), String> {
    for parameter in &job.parameters {
      let current_value = if let Some(value) = parameter.value.clone() {
        value
      } else if let Some(default) = parameter.default.clone() {
        default
      } else {
        continue;
      };

      let id = parameter.get_id();

      match &parameter.kind {
        array_of_strings if array_of_strings == &Vec::<String>::get_type_as_string() => {
          let v = Vec::<String>::parse_value(current_value, &parameter.store)
            .map_err(|e| format!("{:?}", e))?;
          list_of_parameters
            .set_item(id.to_string(), PyList::new(py, v))
            .map_err(|e| py_err_to_string(py, e))?
        }
        string if string == &String::get_type_as_string() => {
          let v =
            String::parse_value(current_value, &parameter.store).map_err(|e| format!("{:?}", e))?;
          list_of_parameters
            .set_item(id.to_string(), v)
            .map_err(|e| py_err_to_string(py, e))?;
        }
        boolean if boolean == &bool::get_type_as_string() => {
          let v =
            bool::parse_value(current_value, &parameter.store).map_err(|e| format!("{:?}", e))?;
          list_of_parameters
            .set_item(id.to_string(), v)
            .map_err(|e| py_err_to_string(py, e))?;
        }
        integer if integer == &i64::get_type_as_string() => {
          let v =
            i64::parse_value(current_value, &parameter.store).map_err(|e| format!("{:?}", e))?;
          list_of_parameters
            .set_item(id.to_string(), v)
            .map_err(|e| py_err_to_string(py, e))?;
        }
        float if float == &f64::get_type_as_string() => {
          let v =
            f64::parse_value(current_value, &parameter.store).map_err(|e| format!("{:?}", e))?;
          list_of_parameters
            .set_item(id.to_string(), v)
            .map_err(|e| py_err_to_string(py, e))?;
        }
        credential if credential == &mcai_worker_sdk::Credential::get_type_as_string() => {
          let credential =
            mcai_worker_sdk::Credential::parse_value(current_value, &parameter.store)
              .map_err(|e| format!("{:?}", e))?;
          list_of_parameters
            .set_item(id.to_string(), credential.value)
            .map_err(|e| py_err_to_string(py, e))?;
        }
        other => {
          return Err(format!(
            "Parameter type not supported by Python SDK: {}",
            other
          ))
        }
      }
    }

    Ok(())
  }
}

static PYTHON_WORKER_EVENT: PythonWorkerEvent = PythonWorkerEvent {};

fn main() {
  start_worker(&PYTHON_WORKER_EVENT);
}
