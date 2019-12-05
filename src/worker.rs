use amqp_worker::worker::{Parameter, ParameterType};
use libloading::Library;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_uint};

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

pub unsafe fn get_parameter_from_worker_parameter(worker_parameter: &WorkerParameter) -> Parameter {
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

pub type GetStringFunc = unsafe fn() -> *const c_char;
pub type GetParametersSizeFunc = unsafe fn() -> c_uint;
pub type GetParametersFunc = unsafe fn(parameters: *mut WorkerParameter);
pub type ProcessFunc = unsafe fn(argc: c_uint, argv: *mut *const c_char) -> c_int;

pub static GET_NAME_FUNCTION: &'static str = "get_name";
pub static GET_SHORT_DESCRIPTION_FUNCTION: &'static str = "get_short_description";
pub static GET_DESCRIPTION_FUNCTION: &'static str = "get_description";
pub static GET_VERSION_FUNCTION: &'static str = "get_version";
pub static GET_PARAMETERS_SIZE_FUNCTION: &'static str = "get_parameters_size";
pub static GET_PARAMETERS_FUNCTION: &'static str = "get_parameters";
pub static PROCESS_FUNCTION: &'static str = "process";

pub unsafe fn get_library_function<'a, T>(
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

pub fn call_worker_process(argc: u32, mut argv: Vec<*const c_char>) -> i32 {
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
