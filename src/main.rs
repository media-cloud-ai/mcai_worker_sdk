#[macro_use]
extern crate log;

use amqp_worker::job::*;
use amqp_worker::start_worker;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::MessageError;
use amqp_worker::MessageEvent;
use pyo3::{prelude::*, types::*};
use semver::Version;
use std::{env, fs};

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

  fn get_git_version(&self) -> Version {
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
      .downcast_ref::<PyList>()
      .unwrap();

    let mut parameters = vec![];

    for item in response.iter() {
      let object = item.downcast_ref::<PyDict>().expect("not a python dict");

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
        .downcast_ref::<PyList>()
        .unwrap();

      let mut parameter_types = vec![];

      for kind in kind_list.iter() {
        let value = kind
          .downcast_ref::<PyString>()
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

  fn process(&self, message: &str) -> Result<JobResult, MessageError> {
    let job = Job::new(message)?;
    debug!("reveived message: {:?}", job);

    match job.check_requirements() {
      Ok(_) => {}
      Err(message) => {
        return Err(message);
      }
    }

    let contents = self.read_python_file();

    let gil = Python::acquire_gil();
    let py = gil.python();
    let python_module = PyModule::from_code(py, &contents, "worker.py", "worker")
      .expect("unable to create the python module");

    if let Err(_error) = python_module.call1("process", (PyString::new(py, message),)) {
      let result = JobResult::new(job.job_id, JobStatus::Error, vec![])
        .with_message("Raised an exception in python.".to_string());

      Err(MessageError::ProcessingError(result))
    } else {
      Ok(JobResult::new(job.job_id, JobStatus::Completed, vec![]))
    }
  }
}

static PYTHON_WORKER_EVENT: PythonWorkerEvent = PythonWorkerEvent {};

fn main() {
  start_worker(&PYTHON_WORKER_EVENT);
}
