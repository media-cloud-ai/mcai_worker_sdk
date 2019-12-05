extern crate libc;
extern crate libloading;
#[macro_use]
extern crate log;

use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};

use amqp_worker::job::*;
use amqp_worker::start_worker;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::MessageEvent;
use amqp_worker::Parameter::*;
use amqp_worker::{MessageError, ParametersContainer};
use libloading::Library;
use semver::Version;

macro_rules! get_c_string {
  ($name:expr) => {
    CString::from(CStr::from_ptr($name))
      .into_string()
      .expect("cannot convert C string to String")
  };
}

fn get_parameter_type_from_c_str(c_str: &CStr) -> ParameterType {
  match c_str.to_str() {
    Ok(c_str) => {
      // keep string quotes in string to json deserializer
      let json_string = format!("{:?}", c_str);
      match serde_json::from_str(&json_string) {
        Ok(parameter_type) => parameter_type,
        Err(msg) => panic!(
          "unable to deserialize worker parameter type {:?}: {:?}",
          json_string, msg
        ),
      }
    }
    Err(msg) => panic!("unable to parse worker parameter type: {:?}", msg),
  }
}

unsafe fn get_parameter_from_worker_parameter(worker_parameter: &WorkerParameter) -> Parameter {
  let identifier = get_c_string!(worker_parameter.identifier);
  let label = get_c_string!(worker_parameter.label);
  let kind_list: &[*const c_char] =
    std::slice::from_raw_parts(worker_parameter.kind, worker_parameter.kind_size);
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

#[repr(C)]
#[derive(Debug)]
pub struct WorkerParameter {
  pub identifier: *const c_char,
  pub label: *const c_char,
  pub kind_size: usize,
  pub kind: *const *const c_char,
  pub required: i32,
}

type GetStringFunc = unsafe fn() -> *const c_char;
type GetParametersSizeFunc = unsafe fn() -> c_uint;
type GetParametersFunc = unsafe fn(parameters: *mut WorkerParameter);
type ProcessFunc = unsafe fn(argc: c_uint, argv: *mut *const c_char) -> c_int;

static GET_NAME_FUNCTION: &'static str = "get_name";
static GET_SHORT_DESCRIPTION_FUNCTION: &'static str = "get_short_description";
static GET_DESCRIPTION_FUNCTION: &'static str = "get_description";
static GET_VERSION_FUNCTION: &'static str = "get_version";
static GET_PARAMETERS_SIZE_FUNCTION: &'static str = "get_parameters_size";
static GET_PARAMETERS_FUNCTION: &'static str = "get_parameters";
static PROCESS_FUNCTION: &'static str = "process";

unsafe fn get_library_function<'a, T>(
  library: &'a Library,
  func_name: &str,
) -> libloading::Symbol<'a, T> {
  library
    .get(func_name.as_bytes())
    .map_err(|error| {
      panic!(format!(
        "Could not find function '{:?}' from worker library: {:?}",
        func_name, error
      ))
    })
    .unwrap()
}

fn get_worker_function_string_value(function_name: &str) -> String {
  let library = std::env::var("WORKER_LIB").unwrap_or("libworker.so".to_string());
  match libloading::Library::new(library) {
    Err(error) => panic!(format!(
      "Could not load worker dynamic library: {:?}",
      error
    )),
    Ok(worker_lib) => unsafe {
      let get_string_func: libloading::Symbol<GetStringFunc> =
        get_library_function(&worker_lib, function_name);
      get_c_string!(get_string_func())
    },
  }
}

fn call_worker_process(argc: u32, mut argv: Vec<*const c_char>) -> i32 {
  let library = std::env::var("WORKER_LIB").unwrap_or("libworker.so".to_string());
  match libloading::Library::new(library) {
    Err(error) => panic!(format!(
      "Could not load worker dynamic library: {:?}",
      error
    )),
    Ok(worker_lib) => unsafe {
      let process_func: libloading::Symbol<ProcessFunc> =
        get_library_function(&worker_lib, PROCESS_FUNCTION);
      process_func(argc, argv.as_mut_ptr())
    },
  }
}

#[derive(Debug)]
struct CWorkerEvent {}

impl MessageEvent for CWorkerEvent {
  fn get_name(&self) -> String {
    get_worker_function_string_value(GET_NAME_FUNCTION)
  }

  fn get_short_description(&self) -> String {
    get_worker_function_string_value(GET_SHORT_DESCRIPTION_FUNCTION)
  }

  fn get_description(&self) -> String {
    get_worker_function_string_value(GET_DESCRIPTION_FUNCTION)
  }

  fn get_version(&self) -> Version {
    let version = get_worker_function_string_value(GET_VERSION_FUNCTION);
    Version::parse(&version).unwrap_or_else(|_| {
      panic!(
        "unable to parse version {} (please use SemVer format)",
        version
      )
    })
  }

  fn get_git_version(&self) -> Version {
    // TODO get real git version?
    self.get_version()
  }

  fn get_parameters(&self) -> Vec<Parameter> {
    let mut parameters = vec![];

    let library = std::env::var("WORKER_LIB").unwrap_or("libworker.so".to_string());
    match libloading::Library::new(library) {
      Err(error) => panic!(format!(
        "Could not load worker dynamic library: {:?}",
        error
      )),
      Ok(worker_lib) => unsafe {
        let get_parameters_size_func: libloading::Symbol<GetParametersSizeFunc> =
          get_library_function(&worker_lib, GET_PARAMETERS_SIZE_FUNCTION);
        let parameters_size = get_parameters_size_func() as usize;
        let worker_parameters =
          libc::malloc(std::mem::size_of::<WorkerParameter>() * parameters_size)
            as *mut WorkerParameter;

        let get_parameters_func: libloading::Symbol<GetParametersFunc> =
          get_library_function(&worker_lib, GET_PARAMETERS_FUNCTION);
        get_parameters_func(worker_parameters);

        let worker_parameters_parts =
          std::slice::from_raw_parts(worker_parameters, parameters_size);
        for worker_parameter in worker_parameters_parts {
          parameters.push(get_parameter_from_worker_parameter(worker_parameter));
        }

        libc::free(worker_parameters as *mut libc::c_void);
      },
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

    let mut list_of_parameters: Vec<String> = Vec::new();

    for parameter in job.get_parameters() {
      match parameter {
        ArrayOfStringsParam { default, value, .. } => {
          if let Some(v) = value {
            for val in v {
              list_of_parameters.push(val.to_string());
            }
          } else if let Some(v) = default {
            for val in v {
              list_of_parameters.push(val.to_string());
            }
          }
        }
        BooleanParam { id, default, value } => {
          if let Some(v) = value {
            if *v {
              list_of_parameters.push(id.to_string());
            }
          } else if let Some(v) = default {
            if *v {
              list_of_parameters.push(id.to_string());
            }
          }
        }
        CredentialParam { id, default, value } => {
          let credential_key = if let Some(v) = value {
            Some(v)
          } else if let Some(v) = default {
            Some(v)
          } else {
            None
          };

          if let Some(credential_key) = credential_key {
            let credential = amqp_worker::Credential {
              key: credential_key.to_string(),
            };
            if let Ok(retrieved_value) = credential.request_value(&job) {
              list_of_parameters.push(id.to_string());
              list_of_parameters.push(retrieved_value);
            } else {
              error!("unable to retrieve the credential value");
            }
          } else {
            error!("no value or default for the credential value");
          }
        }
        IntegerParam { default, value, .. } => {
          if let Some(v) = value {
            list_of_parameters.push(format!("{:?}", v));
          } else if let Some(v) = default {
            list_of_parameters.push(format!("{:?}", v));
          }
        }
        RequirementParam { .. } => {
          // do nothing
        }
        StringParam { default, value, .. } => {
          if let Some(v) = value {
            list_of_parameters.push(v.to_string());
          } else if let Some(v) = default {
            list_of_parameters.push(v.to_string());
          }
        }
      }
    }

    let argc = list_of_parameters.len() as u32;
    debug!("Arguments (length: {:?}): {:?}", argc, list_of_parameters);
    let argv: Vec<*const c_char> = list_of_parameters
      .iter()
      .map(|arg| arg.as_ptr() as *const c_char)
      .collect();

    let return_code = call_worker_process(argc, argv);
    debug!("Returned code: {:?}", return_code);
    match return_code {
      0 => Ok(JobResult::new(job.job_id, JobStatus::Completed, vec![])),
      _ => {
        let result = JobResult::new(job.job_id, JobStatus::Error, vec![]).with_message(format!(
          "Worker process returned error code: {:?}",
          return_code
        ));
        Err(MessageError::ProcessingError(result))
      }
    }
  }
}

static C_WORKER_EVENT: CWorkerEvent = CWorkerEvent {};

fn main() {
  start_worker(&C_WORKER_EVENT);
}

#[test]
pub fn test_c_binding_worker_info() {
  let name = C_WORKER_EVENT.get_name();
  let short_description = C_WORKER_EVENT.get_short_description();
  let description = C_WORKER_EVENT.get_description();
  let version = C_WORKER_EVENT.get_version();
  let git_version = C_WORKER_EVENT.get_git_version();

  assert_eq!(name, "my_c_worker".to_string());
  assert_eq!(short_description, "My C Worker".to_string());
  assert_eq!(
    description,
    "This is my long description \nover multilines".to_string()
  );
  assert_eq!(version.to_string(), "0.1.0".to_string());
  assert_eq!(git_version, version);

  let parameters = C_WORKER_EVENT.get_parameters();
  assert_eq!(1, parameters.len());
  let expected_parameter = Parameter {
    identifier: "my_parameter".to_string(),
    label: "My parameter".to_string(),
    kind: vec![ParameterType::String],
    required: true,
  };
  assert_eq!(expected_parameter.identifier, parameters[0].identifier);
  assert_eq!(expected_parameter.label, parameters[0].label);
  assert_eq!(expected_parameter.kind.len(), parameters[0].kind.len());

  let parameter_kind =
    serde_json::to_string(&parameters[0].kind[0]).expect("cannot serialize parameter kind");
  let expected_kind =
    serde_json::to_string(&expected_parameter.kind[0]).expect("cannot serialize parameter kind");
  assert_eq!(expected_kind, parameter_kind);
  assert_eq!(expected_parameter.required, parameters[0].required);
}

#[test]
pub fn test_c_binding_process() {
  let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];

  let argc = args.len();
  let argv: Vec<*const c_char> = args
    .iter()
    .map(|arg| arg.as_ptr() as *const c_char)
    .collect();

  assert_eq!(argc, argv.len());
  let returned_code = call_worker_process(argc as u32, argv);
  assert_eq!(0, returned_code);
}

#[test]
pub fn test_c_binding_failing_process() {
  let args = vec!["arg1".to_string(), "arg2".to_string()];

  let argc = args.len();

  let argv: Vec<*const c_char> = args
    .iter()
    .map(|arg| arg.as_ptr() as *const c_char)
    .collect();

  let returned_code = call_worker_process(argc as u32, argv);
  assert_eq!(1, returned_code);
}

#[test]
pub fn test_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "human",
        "type": "string",
        "value": "--human"
      },
      {
        "id": "verbose",
        "type": "string",
        "value": "--verbose"
      },
      {
        "id": "path",
        "type": "string",
        "value": "/path/to/file"
      }
    ]
  }"#;

  let result = C_WORKER_EVENT.process(message);
  assert!(result.is_ok());
  let job_result = result.unwrap();
  assert_eq!(123, job_result.job_id);
  assert_eq!(JobStatus::Completed, job_result.status);
}

#[test]
pub fn test_failing_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "path",
        "type": "string",
        "value": "/path/to/file"
      }
    ]
  }"#;

  let result = C_WORKER_EVENT.process(message);
  assert!(result.is_err());
  let _message_error = result.unwrap_err();
}
