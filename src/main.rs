extern crate libc;
#[macro_use]
extern crate log;

use std::ffi::{CStr, CString, IntoStringError};
use std::os::raw::{c_char, c_int, c_uint};

use amqp_worker::{MessageError, ParametersContainer};
use amqp_worker::job::*;
use amqp_worker::MessageEvent;
use amqp_worker::Parameter::*;
use amqp_worker::start_worker;
use amqp_worker::worker::{Parameter, ParameterType};
use semver::Version;

#[repr(C)]
#[derive(Debug)]
pub struct WorkerParameter {
  pub identifier: *const c_char,
  pub label: *const c_char,
  pub kind_size: usize,
  pub kind: *const (*const c_char),
  pub required: i32,
}

extern "C" {
  fn get_name() -> *const c_char;
  fn get_short_description() -> *const c_char;
  fn get_description() -> *const c_char;
  fn get_version() -> *const c_char;
  fn get_parameters_size() -> c_uint;
  fn get_parameters(parameters: *mut WorkerParameter);
  fn process(argc: c_uint, argv: *mut (*const c_char)) -> c_int;
}

fn get_parameter_type_from_c_str(c_str: &CStr) -> ParameterType {
  match c_str.to_str() {
    Ok(c_str) => {
      // keep string quotes in string to json deserializer
      let json_string = format!("{:?}", c_str);
      match serde_json::from_str(&json_string) {
        Ok(param_type) => param_type,
        Err(e) => panic!("unable to deserialize worker parameter type {:?}: {:?}", json_string, e)
      }
    }
    Err(e) => panic!("unable to parse worker parameter type: {:?}", e)
  }
}

unsafe fn get_string_from_c_string(c_str: *const c_char) -> Result<String, IntoStringError> {
  CString::from(CStr::from_ptr(c_str)).into_string()
}

unsafe fn get_parameter_from_worker_parameter(worker_parameter: &WorkerParameter) -> Parameter {
  let identifier = get_string_from_c_string(worker_parameter.identifier).expect("unable to parse worker parameter identifier");
  let label = get_string_from_c_string(worker_parameter.label).expect("unable to parse worker parameter label");
  let kind_list: &[*const c_char] = std::slice::from_raw_parts(worker_parameter.kind, worker_parameter.kind_size);
  let mut parameter_types = vec![];
  for kind in kind_list.iter() {
    parameter_types.push(get_parameter_type_from_c_str(CStr::from_ptr(*kind)));
  }
  let required = worker_parameter.required > 0;

  Parameter {
    identifier,
    label,
    kind: parameter_types,
    required,
  }
}


#[derive(Debug)]
struct CWorkerEvent {}

impl MessageEvent for CWorkerEvent {
  fn get_name(&self) -> String {
    let mut name: String;
    unsafe {
      name = get_string_from_c_string(get_name()).expect("unable to get name from C worker");
    }
    name
  }

  fn get_short_description(&self) -> String {
    let mut short_description: String;
    unsafe {
      short_description = get_string_from_c_string(get_short_description()).expect("unable to get short description from C worker");
    }
    short_description
  }

  fn get_description(&self) -> String {
    let mut description: String;
    unsafe {
      description = get_string_from_c_string(get_description()).expect("unable to get description from C worker");
    }
    description
  }

  fn get_version(&self) -> Version {
    let version: String;
    unsafe {
      version = get_string_from_c_string(get_version()).expect("unable to get version from C worker");
    }
    Version::parse(version.as_str())
      .expect(&format!("unable to parse version {} (please use SemVer format)", version))
  }

  fn get_git_version(&self) -> Version {
    // TODO get real git version?
    self.get_version()
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    let worker_parameters: *mut WorkerParameter;
    let mut parameters = vec![];

    unsafe {
      let parameters_size = get_parameters_size() as usize;
      worker_parameters = libc::malloc(std::mem::size_of::<WorkerParameter>() * parameters_size) as *mut WorkerParameter;
      get_parameters(worker_parameters);
      let worker_parameters_parts = std::slice::from_raw_parts(worker_parameters, parameters_size);
      for worker_parameter in worker_parameters_parts.iter() {
        parameters.push(get_parameter_from_worker_parameter(worker_parameter));
      }
      libc::free(worker_parameters as *mut libc::c_void);
    }

    parameters
  }

  fn process(&self, message: &str) -> Result<JobResult, MessageError> {
    let job = Job::new(message)?;
    debug!("received message: {:?}", job);

    match job.check_requirements() {
      Ok(_) => {}
      Err(message) => {
        return Err(message);
      }
    }

    Ok(JobResult::new(job.job_id, JobStatus::Completed, vec![]))
  }
}

static C_WORKER_EVENT: CWorkerEvent = CWorkerEvent {};

fn main() {
  start_worker(&C_WORKER_EVENT);
}

