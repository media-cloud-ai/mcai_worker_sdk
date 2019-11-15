#[macro_use]
extern crate log;

use amqp_worker::job::*;
use amqp_worker::start_worker;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::MessageError;
use amqp_worker::MessageEvent;
use amqp_worker::Parameter::*;
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
    let traceback = py.import("traceback").unwrap();
    let python_module = PyModule::from_code(py, &contents, "worker.py", "worker")
      .expect("unable to create the python module");

    let list_of_parameters = PyDict::new(py);
    if let Err(error) = self.build_parameters(&job, &py, list_of_parameters) {
      let locals = [("error", error)].into_py_dict(py);

      let error_msg = py
        .eval("repr(error)", None, Some(locals))
        .expect("Unknown python error, unable to get the error message")
        .to_string();

      let result = JobResult::new(job.job_id, JobStatus::Error, vec![]).with_message(error_msg);
      return Err(MessageError::ProcessingError(result));
    }

    let parameters = PyTuple::new(py, vec![list_of_parameters]);

    if let Err(error) = python_module.call1("process", parameters) {
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

      let result = JobResult::new(job.job_id, JobStatus::Error, vec![]).with_message(error_message);
      Err(MessageError::ProcessingError(result))
    } else {
      Ok(JobResult::new(job.job_id, JobStatus::Completed, vec![]))
    }
  }
}

impl PythonWorkerEvent {
  fn build_parameters(&self, job: &Job, py: &Python, list_of_parameters: &PyDict) -> Result<(), PyErr> {
    for parameter in &job.parameters {
      match parameter {
        ArrayOfStringsParam{id, default, value} => {
          if let Some(v) = value {
            list_of_parameters.set_item(id.to_string(), PyList::new(*py, v))?;
          } else if let Some(v) = default {
            list_of_parameters.set_item(id.to_string(), PyList::new(*py, v))?;
          }
        }
        BooleanParam{id, default, value} => {
          if let Some(v) = value {
            list_of_parameters.set_item(id.to_string(), v)?;
          } else if let Some(v) = default {
            list_of_parameters.set_item(id.to_string(), v)?;
          }
        },
        CredentialParam{id, default, value} => {
          let credential_key =
            if let Some(v) = value {
              Some(v)
            } else if let Some(v) = default {
              Some(v)
            } else {
              None
            };

          if let Some(credential_key) = credential_key {
            let credential = amqp_worker::Credential{key: credential_key.to_string()};
            if let Ok(retrieved_value) = credential.request_value(&job) {
              list_of_parameters.set_item(id.to_string(), retrieved_value)?;
            } else {
              error!("unable to retrieve the credential value");
            }
          } else {
            error!("no value or default for the credential value");
          }
        },
        IntegerParam{id, default, value} => {
          if let Some(v) = value {
            list_of_parameters.set_item(id.to_string(), v)?;
          } else if let Some(v) = default {
            list_of_parameters.set_item(id.to_string(), v)?;
          }
        },
        RequirementParam{..} => {
          // do nothing
        },
        StringParam{id, default, value} => {
          if let Some(v) = value {
            list_of_parameters.set_item(id.to_string(), v)?;
          } else if let Some(v) = default {
            list_of_parameters.set_item(id.to_string(), v)?;
          }
        },
      }
    }

    Ok(())
  }
}

static PYTHON_WORKER_EVENT: PythonWorkerEvent = PythonWorkerEvent {};

fn main() {
  start_worker(&PYTHON_WORKER_EVENT);
}
