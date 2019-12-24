
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};

use amqp_worker::job::Job;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::ParametersContainer;
use libloading::Library;
use crate::constants;
use crate::process_return::ProcessReturn;

macro_rules! get_c_string {
  ($name:expr) => {
    if $name.is_null() {
      "".to_string()
    } else {
      std::str::from_utf8_unchecked(CStr::from_ptr($name).to_bytes()).to_string()
    }
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
type LoggerCallback = extern "C" fn(*const c_char, *const c_char);
type ProcessFunc = unsafe fn(
  job: *mut c_void,
  callback: GetParameterValueCallback,
  logger: LoggerCallback,
  output_message: &*const c_char,
  output_paths: &*mut *const c_char
) -> c_int;

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
    std::ptr::null()
  };

  // reset job parameters pointer
  c_worker_job = Box::into_raw(job_params_ptrs) as *mut c_void;
  param_value
}

extern "C" fn logger(level: *const c_char, raw_value: *const c_char) {
  unsafe {
    let level = get_c_string!(level);
    let value = get_c_string!(raw_value);

    match level.as_str() {
      "trace" => {trace!("[Worker] {}", value);},
      "debug" => {debug!("[Worker] {}", value);},
      "info" => {info!("[Worker] {}", value);},
      "error" => {error!("[Worker] {}", value);},
      _ => {}
    }
  }
}

/************************
 *   Utility functions
 ************************/

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
        get_library_function(&worker_lib, constants::GET_PARAMETERS_SIZE_FUNCTION)
          .unwrap_or_else(|error| panic!(error));
      let parameters_size = get_parameters_size_func() as usize;

      // Allocate a C array to retrieve the worker parameters
      let worker_parameters = libc::malloc(std::mem::size_of::<WorkerParameter>() * parameters_size)
        as *mut WorkerParameter;

      let get_parameters_func: libloading::Symbol<GetParametersFunc> =
        get_library_function(&worker_lib, constants::GET_PARAMETERS_FUNCTION)
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
      match get_library_function(&worker_lib, constants::PROCESS_FUNCTION)
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

          let message_ptr = std::ptr::null();

          let mut output_paths_ptr = vec![std::ptr::null()];
          let ptr = output_paths_ptr.as_mut_ptr();

          // Call C worker process function
          let return_code = process_func(
            job_params_ptrs_ptr as *mut c_void,
            get_parameter_value,
            logger,
            &message_ptr,
            &ptr
          );

          let mut output_paths = vec![];

          if !ptr.is_null() {
            let mut offset = 0;
            loop {
              let cur_ptr = *ptr.offset(offset);
              if cur_ptr.is_null() {
                break;
              }

              output_paths.push(get_c_string!(cur_ptr));

              libc::free(cur_ptr as *mut libc::c_void);
              offset += 1;
            }

            if offset > 0 {
              libc::free(ptr as *mut libc::c_void);
            }
          }

          // Retrieve message as string and free pointer
          let mut message = "".to_string();
          if !message_ptr.is_null() {
            message = get_c_string!(message_ptr);
            libc::free(message_ptr as *mut libc::c_void);
          }

          ProcessReturn::new(return_code, &message)
            .with_output_paths(output_paths)
        }
        Err(error) => ProcessReturn::new_error(&format!(
          "Could not access {:?} function from worker library: {:?}",
          constants::PROCESS_FUNCTION, error
        )),
      }
    },
    Err(error) => ProcessReturn::new_error(&format!(
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
  assert_eq!(returned_code.get_code(), 0);
  assert_eq!(returned_code.get_message(), "Everything worked well!");
  assert_eq!(returned_code.get_output_paths(), &vec!["/path/out.mxf".to_string()]);
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
  assert_eq!(returned_code.get_code(), 1);
  assert_eq!(returned_code.get_message(), "Something went wrong...");
  assert!(returned_code.get_output_paths().is_empty());
}
