use amqp_worker::job::Job;
use amqp_worker::worker::{Parameter, ParameterType};
use amqp_worker::ParametersContainer;
use libloading::Library;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint, c_void};

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
type ProcessFunc =
  unsafe fn(job: *mut c_void, callback: GetParameterValueCallback, logger: LogCallback) -> c_int;

pub static GET_NAME_FUNCTION: &'static str = "get_name";
pub static GET_SHORT_DESCRIPTION_FUNCTION: &'static str = "get_short_description";
pub static GET_DESCRIPTION_FUNCTION: &'static str = "get_description";
pub static GET_VERSION_FUNCTION: &'static str = "get_version";
pub static GET_PARAMETERS_SIZE_FUNCTION: &'static str = "get_parameters_size";
pub static GET_PARAMETERS_FUNCTION: &'static str = "get_parameters";
pub static PROCESS_FUNCTION: &'static str = "process";

extern "C" fn get_parameter_value(
  c_worker_job: *mut c_void,
  parameter_id: *const c_char,
) -> *const c_char {
  let job_params_ptrs: Box<HashMap<String, *const c_char>> =
    unsafe { Box::from_raw(c_worker_job as *mut HashMap<String, *const c_char>) };
  let key = unsafe { get_c_string!(parameter_id) };

  if let Some(value) = job_params_ptrs.get(&key) {
    *value
  } else {
    panic!("No worker_job parameter for id: {}.", key);
  }
}

extern "C" fn log(value: *const c_char) {
  unsafe {
    info!("[Worker] {}", get_c_string!(value));
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

pub fn get_worker_function_string_value(function_name: &str) -> String {
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

pub fn get_worker_parameters() -> Vec<Parameter> {
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
      let worker_parameters = libc::malloc(std::mem::size_of::<WorkerParameter>() * parameters_size)
        as *mut WorkerParameter;

      let get_parameters_func: libloading::Symbol<GetParametersFunc> =
        get_library_function(&worker_lib, GET_PARAMETERS_FUNCTION);
      get_parameters_func(worker_parameters);

      let worker_parameters_parts = std::slice::from_raw_parts(worker_parameters, parameters_size);
      for worker_parameter in worker_parameters_parts {
        parameters.push(get_parameter_from_worker_parameter(worker_parameter));
      }

      libc::free(worker_parameters as *mut libc::c_void);
    },
  }

  parameters
}

pub fn call_worker_process(job: Job) -> i32 {
  let library = std::env::var("WORKER_LIB").unwrap_or("libworker.so".to_string());
  match libloading::Library::new(library) {
    Err(error) => panic!(format!(
      "Could not load worker dynamic library: {:?}",
      error
    )),
    Ok(worker_lib) => unsafe {
      let process_func: libloading::Symbol<ProcessFunc> =
        get_library_function(&worker_lib, PROCESS_FUNCTION);

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

      // Call C worker process function
      process_func(job_params_ptrs_ptr as *mut c_void, get_parameter_value, log)
    },
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
  assert_eq!(0, returned_code);
}

