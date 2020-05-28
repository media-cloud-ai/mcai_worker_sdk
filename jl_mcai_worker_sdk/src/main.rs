use crate::helpers::get_destination_paths;
use mcai_worker_sdk::{
  error,
  job::*,
  publish_job_progression, start_worker,
  worker::{Parameter, ParameterType},
  Channel, Credential, MessageError, MessageEvent,
  Parameter::*,
  Version,
};
use jlrs::prelude::*;
use std::{env, fs};

mod helpers;

#[derive(Debug)]
struct JuliaWorkerEvent {}

impl JuliaWorkerEvent {
    fn read_julia_file(&self) -> String {
        let filename = env::var("JULIA_WORKER_FILENAME").unwrap_or_else(|_| "worker.jl".to_string());

        fs::read_to_string(&filename)
          .unwrap_or_else(|_| panic!("unable to open and read file: {}", filename))
    }
}

static JULIA_WORKER_EVENT: JuliaWorkerEvent = JuliaWorkerEvent {};

fn main() {
    start_worker(&JULIA_WORKER_EVENT);
}

impl MessageEvent for JuliaWorkerEvent {
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
    let contents = self.read_julia_file();

    let mut julia = unsafe { Julia::init(16).unwrap() };
    julia.include("worker.jl");

    julia.frame(3, |global, frame| {

    }

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
    channel: Option<&Channel>,
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
      let locals = [("error", error)].into_py_dict(py);

      let error_msg = py
        .eval("repr(error)", None, Some(locals))
        .expect("Unknown python error, unable to get the error message")
        .to_string();

      let result = job_result
        .with_status(JobStatus::Error)
        .with_message(&error_msg);
      return Err(MessageError::ProcessingError(result));
    }

    let callback_handle = CallbackHandle {
      channel: channel.unwrap().clone(),
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
