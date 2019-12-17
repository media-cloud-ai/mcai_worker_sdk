use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};

use amqp_worker::job::Job;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::ParametersContainer;
use libloading::Library;

thread_local!(static LAST_ERROR: RefCell<Option<String>> = RefCell::new(None));

macro_rules! handle_error {
  ($name:expr) => {
    LAST_ERROR.with(|last_error| {
      last_error.replace(Some($name));
    });
  };
}

macro_rules! get_c_string {
  ($name:expr) => {
    CString::from(CStr::from_ptr($name))
      .into_string()
      .expect("cannot convert C string to String")
  };
}

/************************
 *      C Binding
 ************************/

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

type GetParameterValueCallback = extern "C" fn(*mut c_void, *const c_char) -> *const c_char;
type LogCallback = extern "C" fn(*const c_char);
type ProcessFunc = unsafe fn(
  job: *mut c_void,
  callback: GetParameterValueCallback,
  check_error: CheckLastError,
  logger: LogCallback,
  output_message: *mut c_char,
) -> c_int;

type CheckLastError = extern "C" fn() -> c_int;

pub static GET_NAME_FUNCTION: &str = "get_name";
pub static GET_SHORT_DESCRIPTION_FUNCTION: &str = "get_short_description";
pub static GET_DESCRIPTION_FUNCTION: &str = "get_description";
pub static GET_VERSION_FUNCTION: &str = "get_version";
pub static GET_PARAMETERS_SIZE_FUNCTION: &str = "get_parameters_size";
pub static GET_PARAMETERS_FUNCTION: &str = "get_parameters";
pub static PROCESS_FUNCTION: &str = "process";

extern "C" fn check_error() -> c_int {
  let last_error = LAST_ERROR.with(|last_error| last_error.replace(None));
  if let Some(error_message) = last_error {
    return_with_error(error_message).code
  } else {
    0
  }
}

#[allow(unused_assignments)]
extern "C" fn get_parameter_value(
  mut c_worker_job: *mut c_void,
  parameter_id: *const c_char,
) -> *const c_char {
  let job_params_ptrs: Box<HashMap<String, *const c_char>> =
    unsafe { Box::from_raw(c_worker_job as *mut HashMap<String, *const c_char>) };
  let key = unsafe { get_c_string!(parameter_id) };
  debug!("Get parameter value from id: {:?}", key);
  let param_value = if let Some(value) = job_params_ptrs.get(&key) {
    *value
  } else {
    handle_error!(format!("No worker_job parameter for id: {}.", key));
    std::ptr::null()
  };
  // reset job parameters pointer
  c_worker_job = Box::into_raw(job_params_ptrs) as *mut c_void;
  param_value
}

extern "C" fn log(value: *const c_char) {
  unsafe {
    debug!("[Worker] {}", get_c_string!(value));
  }
}

/************************
 *   Utility functions
 ************************/

#[derive(Debug)]
pub struct ProcessReturn {
  pub code: i32,
  pub message: String,
}

fn return_with_error(message: String) -> ProcessReturn {
  ProcessReturn { code: 1, message }
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

fn get_library_file_path() -> String {
  std::env::var("WORKER_LIBRARY_FILE").unwrap_or_else(|_| "libworker.so".to_string())
}

unsafe fn get_library_function<'a, T>(
  library: &'a Library,
  func_name: &str,
) -> Result<libloading::Symbol<'a, T>, String> {
  library.get(func_name.as_bytes()).map_err(|error| {
    format!(
      "Could not find function '{:?}' from worker library: {:?}",
      func_name, error
    )
  })
}

pub fn get_worker_function_string_value(function_name: &str) -> String {
  match libloading::Library::new(get_library_file_path()) {
    Ok(worker_lib) => unsafe {
      let get_string_func: libloading::Symbol<GetStringFunc> =
        get_library_function(&worker_lib, function_name).unwrap_or_else(|error| panic!(error));
      get_c_string!(get_string_func())
    },
    Err(error) => panic!(format!(
      "Could not load worker dynamic library: {:?}",
      error
    )),
  }
}

pub fn get_worker_parameters() -> Vec<Parameter> {
  let mut parameters = vec![];
  match libloading::Library::new(get_library_file_path()) {
    Ok(worker_lib) => unsafe {
      // Retrieve number of parameters from the worker getter function
      let get_parameters_size_func: libloading::Symbol<GetParametersSizeFunc> =
        get_library_function(&worker_lib, GET_PARAMETERS_SIZE_FUNCTION)
          .unwrap_or_else(|error| panic!(error));
      let parameters_size = get_parameters_size_func() as usize;

      // Allocate a C array to retrieve the worker parameters
      let worker_parameters = libc::malloc(std::mem::size_of::<WorkerParameter>() * parameters_size)
        as *mut WorkerParameter;

      let get_parameters_func: libloading::Symbol<GetParametersFunc> =
        get_library_function(&worker_lib, GET_PARAMETERS_FUNCTION)
          .unwrap_or_else(|error| panic!(error));
      get_parameters_func(worker_parameters);

      // Convert the retrieved worker parameters to AMQP Parameter instances
      let worker_parameters_parts = std::slice::from_raw_parts(worker_parameters, parameters_size);
      for worker_parameter in worker_parameters_parts {
        parameters.push(get_parameter_from_worker_parameter(worker_parameter));
      }

      // Free parameters C array
      libc::free(worker_parameters as *mut libc::c_void);
    },
    Err(error) => panic!(format!(
      "Could not load worker dynamic library: {:?}",
      error
    )),
  }

  parameters
}

pub fn call_worker_process(job: Job) -> ProcessReturn {
  let library = get_library_file_path();
  debug!("Call worker process from library: {}", library);
  match libloading::Library::new(library) {
    Ok(worker_lib) => unsafe {
      match get_library_function(&worker_lib, PROCESS_FUNCTION)
        as Result<libloading::Symbol<ProcessFunc>, String>
      {
        Ok(process_func) => {
          // Get job parameters C pointers, and cache references
          let mut job_params_ptrs = HashMap::new();
          let mut job_param_c_string_values_cache: Vec<CString> = vec![];
          job.get_parameters_as_map().iter().for_each(|(key, value)| {
            let c_string = CString::new(value.as_str()).unwrap();
            job_param_c_string_values_cache.push(c_string);
            job_params_ptrs.insert(
              key.clone(),
              job_param_c_string_values_cache.last().unwrap().as_ptr(),
            );
          });

          // Get job parameters map pointer
          let boxed_job_params_ptrs = Box::new(job_params_ptrs);
          let job_params_ptrs_ptr = Box::into_raw(boxed_job_params_ptrs);

          // Get output message pointer
          let message_ptr = libc::malloc(1024*1024) as *mut c_char; // 1MB max. sized message

          // Call C worker process function
          let return_code = process_func(
            job_params_ptrs_ptr as *mut c_void,
            get_parameter_value,
            check_error,
            log,
            message_ptr,
          );

          // Retrieve message as string and free pointer
          let message = get_c_string!(message_ptr);
          libc::free(message_ptr as *mut libc::c_void);

          ProcessReturn {
            code: return_code,
            message,
          }
        }
        Err(error) => return_with_error(format!(
          "Could not access {:?} function from worker library: {:?}",
          PROCESS_FUNCTION, error
        )),
      }
    },
    Err(error) => return_with_error(format!(
      "Could not load worker dynamic library: {:?}",
      error
    )),
  }
}

#[test]
pub fn test_c_binding_process() {
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

  let job = Job::new(message).unwrap();
  let returned_code = call_worker_process(job);
  assert_eq!(0, returned_code.code);
  assert_eq!("Everything worked well!", returned_code.message);
}

#[test]
pub fn test_c_binding_failing_process() {
  let message = r#"{
    "job_id": 123,
    "parameters": [
      {
        "id": "not_the_expected_path_parameter",
        "type": "string",
        "value": "value"
      }
    ]
  }"#;

  let job = Job::new(message).unwrap();
  let returned_code = call_worker_process(job);
  assert_eq!(1, returned_code.code);
  assert_eq!("Something went wrong...", returned_code.message);
}
