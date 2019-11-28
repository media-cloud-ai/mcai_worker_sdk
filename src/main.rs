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

    let mut list_of_parameters: Vec<String> = Vec::new();
    let parameters = job.get_parameters();
    for parameter in parameters {
      match parameter {
        ArrayOfStringsParam { id: _, default, value } => {
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
        IntegerParam { id: _, default, value } => {
          if let Some(v) = value {
            list_of_parameters.push(format!("{:?}", v));
          } else if let Some(v) = default {
            list_of_parameters.push(format!("{:?}", v));
          }
        }
        RequirementParam { .. } => {
          // do nothing
        }
        StringParam { id: _, default, value } => {
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
    let mut argv: Vec<*const c_char> = list_of_parameters.iter().map(|arg| arg.as_ptr() as *const c_char).collect();

    unsafe {
      let return_code = process(argc, argv.as_mut_ptr());
      debug!("Returned code: {:?}", return_code);

      match return_code {
        0 => Ok(JobResult::new(job.job_id, JobStatus::Completed, vec![])),
        _ => {
          let result =
            JobResult::new(job.job_id, JobStatus::Error, vec![])
              .with_message(format!("Worker process returned error code: {:?}", return_code));
          Err(MessageError::ProcessingError(result))
        }
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
  unsafe {
    let name = get_string_from_c_string(get_name()).expect("cannot convert C string to String");
    let expected_name = "my_c_worker".to_string();
    assert_eq!(expected_name, name);

    let short_description = get_string_from_c_string(get_short_description()).expect("cannot convert C string to String");
    let expected_short_description = "My C Worker".to_string();
    assert_eq!(expected_short_description, short_description);

    let description = get_string_from_c_string(get_description()).expect("cannot convert C string to String");
    let expected_description = "This is my long description \nover multilines".to_string();
    assert_eq!(expected_description, description);

    let version = get_string_from_c_string(get_version()).expect("cannot convert C string to String");
    let expected_version = "0.1.0".to_string();
    assert_eq!(expected_version, version);

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

    let parameter_kind = serde_json::to_string(&parameters[0].kind[0]).expect("cannot serialize parameter kind");
    let expected_kind = serde_json::to_string(&expected_parameter.kind[0]).expect("cannot serialize parameter kind");
    assert_eq!(expected_kind, parameter_kind);
    assert_eq!(expected_parameter.required, parameters[0].required);
  }
}

#[test]
pub fn test_c_binding_process() {
  unsafe {
    let args = vec!["arg1".to_string(), "arg2".to_string(), "arg3".to_string()];
    let argc = args.len();
    let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr() as *const c_char).collect();
    assert_eq!(argc, argv.len());
    let returned_code = process(argc as u32, argv.as_mut_ptr());
    assert_eq!(0, returned_code);
  }
}

#[test]
pub fn test_c_binding_failing_process() {
  unsafe {
    let args = vec!["arg1".to_string(), "arg2".to_string()];
    let argc = args.len();
    let mut argv: Vec<*const c_char> = args.iter().map(|arg| arg.as_ptr() as *const c_char).collect();
    let returned_code = process(argc as u32, argv.as_mut_ptr());
    assert_eq!(1, returned_code);
  }
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
