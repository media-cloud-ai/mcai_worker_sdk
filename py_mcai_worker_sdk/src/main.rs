#[macro_use]
extern crate serde_derive;

#[cfg(feature = "media")]
use std::sync::{Arc, Mutex};
use std::{env, fs};

use pyo3::{prelude::*, types::*};

use mcai_worker_sdk::{
  info,
  job::{JobResult, JobStatus},
  publish_job_progression, start_worker,
  worker::{Parameter, ParameterType},
  McaiChannel, MessageError, MessageEvent, Result, Version,
};
#[cfg(feature = "media")]
pub use mcai_worker_sdk::{FormatContext, Frame, ProcessResult};

use crate::helpers::get_destination_paths;
#[cfg(feature = "media")]
use crate::helpers::get_stream_indexes;
use crate::helpers::py_err_to_string;
use crate::parameters::{build_parameters, PythonWorkerParameters};

mod helpers;
#[cfg(feature = "media")]
mod media;
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
    let gil = Python::acquire_gil();
    let (py, python_module) = get_python_module(&gil).unwrap();

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

fn get_python_module<'a>(gil: &'a GILGuard) -> Result<(Python<'a>, &'a PyModule)> {
  let python_file_content = PythonWorkerEvent::read_python_file();
  let py = gil.python();
  let python_module = PyModule::from_code(py, &python_file_content, "worker.py", "worker")
    .map_err(|error| {
      MessageError::RuntimeError(format!(
        "unable to create the python module: {}",
        py_err_to_string(py, error)
      ))
    })?;
  Ok((py, python_module))
}

fn call_module_function<'a>(
  py: Python,
  python_module: &'a PyModule,
  function_name: &'a str,
  args: impl IntoPy<Py<PyTuple>>,
) -> std::result::Result<&'a PyAny, String> {
  python_module
    .call1(function_name, args)
    .map_err(move |error| {
      let ptraceback = &error.ptraceback;
      let stacktrace = ptraceback
        .as_ref()
        .map(|tb| {
          let traceback = py.import("traceback").unwrap();
          let locals = [("traceback", traceback)].into_py_dict(py);

          locals.set_item("tb", tb).unwrap();

          py.eval("traceback.format_tb(tb)", None, Some(locals))
            .expect("Unknown python error, unable to get the stacktrace")
            .to_string()
        })
        .unwrap_or_else(|| "Unknown python error, no stackstrace".to_string());

      let error_msg = py_err_to_string(py, error);

      format!("{}\n\nStacktrace:\n{}", error_msg, stacktrace)
    })
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

  fn init(&mut self) -> Result<()> {
    let gil = Python::acquire_gil();
    let (py, python_module) = get_python_module(&gil)?;

    let optional_init_function_name = "init";

    if python_module.get(optional_init_function_name).is_ok() {
      let _result = call_module_function(py, python_module, optional_init_function_name, ())
        .map_err(MessageError::ParameterValueError)?;
    } else {
      info!(
        "No optional '{}' function to call.",
        optional_init_function_name
      );
    }

    Ok(())
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    parameters: PythonWorkerParameters,
    format_context: Arc<Mutex<FormatContext>>,
  ) -> Result<Vec<usize>> {
    let gil = Python::acquire_gil();
    let (py, python_module) = get_python_module(&gil)?;

    let context = media::FormatContext::from(format_context);
    let list_of_parameters = build_parameters(parameters, py)?;

    let response = call_module_function(
      py,
      python_module,
      "init_process",
      (context, list_of_parameters),
    )
    .map_err(MessageError::ParameterValueError)?;
    get_stream_indexes(response)
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    job_result: JobResult,
    stream_index: usize,
    frame: Frame,
  ) -> Result<ProcessResult> {
    let gil = Python::acquire_gil();
    let (py, python_module) = get_python_module(&gil)?;

    let media_frame = media::Frame::from(&frame)?;

    let response = call_module_function(
      py,
      python_module,
      "process_frame",
      (&job_result.get_str_job_id(), stream_index, media_frame),
    )
    .map_err(|error_message| {
      let result = job_result
        .with_status(JobStatus::Error)
        .with_message(&error_message);
      MessageError::ProcessingError(result)
    })?;

    Ok(ProcessResult::new_json(&response.to_string()))
  }

  #[cfg(feature = "media")]
  fn ending_process(&mut self) -> Result<()> {
    let gil = Python::acquire_gil();
    let (py, python_module) = get_python_module(&gil)?;

    let _result = call_module_function(py, python_module, "ending_process", ())
      .map_err(MessageError::ParameterValueError)?;

    Ok(())
  }

  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: PythonWorkerParameters,
    mut job_result: JobResult,
  ) -> Result<JobResult> {
    let gil = Python::acquire_gil();
    let (py, python_module) = get_python_module(&gil)?;

    let list_of_parameters = build_parameters(parameters, py)?;

    let callback_handle = CallbackHandle {
      channel,
      job_id: job_result.get_job_id(),
    };

    let response = call_module_function(
      py,
      python_module,
      "process",
      (callback_handle, list_of_parameters),
    )
    .map_err(|error_message| {
      let result = job_result
        .clone()
        .with_status(JobStatus::Error)
        .with_message(&error_message);
      MessageError::ProcessingError(result)
    })?;

    if let Some(mut destination_paths) = get_destination_paths(response) {
      job_result = job_result.with_destination_paths(&mut destination_paths);
    }

    Ok(job_result.with_status(JobStatus::Completed))
  }
}

static PYTHON_WORKER_EVENT: PythonWorkerEvent = PythonWorkerEvent {};

fn main() {
  start_worker(PYTHON_WORKER_EVENT.clone());
}
