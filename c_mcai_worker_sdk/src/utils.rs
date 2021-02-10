use crate::constants;
use crate::get_c_string;
use crate::parameters::{get_parameter_from_worker_parameter, CWorkerParameters};
use crate::types::{GetParametersFunc, GetParametersSizeFunc, GetStringFunc};
use crate::worker::CWorkerParameter;
use libloading::Library;
use mcai_worker_sdk::prelude::*;
use serde_json::Value;
use std::{
  ffi::CString,
  os::raw::{c_char, c_uchar, c_void},
  ptr::null,
};

#[macro_export]
macro_rules! get_c_string {
  ($name:expr) => {
    if $name.is_null() {
      "".to_string()
    } else {
      std::str::from_utf8_unchecked(std::ffi::CStr::from_ptr($name).to_bytes()).to_string()
    }
  };
}

#[repr(C)]
pub struct Handler {
  pub job_id: Option<u64>,
  pub parameters: Option<CWorkerParameters>,
  pub channel: Option<McaiChannel>,
}

/************************
 *      C Binding
 ************************/

#[allow(unused_assignments)]
pub(crate) extern "C" fn get_parameter_value(
  mut c_handler: *mut c_void,
  parameter_id: *const c_char,
) -> *const c_char {
  if c_handler.is_null() {
    error!("Null handler");
    return null();
  }

  let handler: Box<Handler> = unsafe { Box::from_raw(c_handler as *mut Handler) };

  if let Some(parameters) = handler.parameters.clone() {
    let key = unsafe { get_c_string!(parameter_id) };

    let param_value = if let Some(value) = parameters.parameters.get(&key) {
      let string = match value {
        Value::String(string) => CString::new(string.as_str()).unwrap(),
        Value::Number(number) => CString::new(format!("{}", number.as_i64().unwrap())).unwrap(),
        Value::Bool(boolean) => CString::new(format!("{}", boolean)).unwrap(),
        Value::Array(array) => CString::new(format!("{:?}", array)).unwrap(),
        Value::Object(object) => CString::new(format!("{:?}", object)).unwrap(),
        Value::Null => CString::new(format!("")).unwrap(),
      };

      let string_ptr = string.as_ptr();
      std::mem::forget(string);
      string_ptr
    } else {
      null()
    };

    // reset job parameters pointer
    c_handler = Box::into_raw(handler) as *mut c_void;
    param_value
  } else {
    null()
  }
}

pub(crate) extern "C" fn logger(level: *const c_char, raw_value: *const c_char) {
  unsafe {
    let level = get_c_string!(level);
    let value = get_c_string!(raw_value);

    match level.as_str() {
      "trace" => {
        trace!("[Worker] {}", value);
      }
      "debug" => {
        debug!("[Worker] {}", value);
      }
      "info" => {
        info!("[Worker] {}", value);
      }
      "warn" => {
        warn!("[Worker] {}", value);
      }
      "error" => {
        error!("[Worker] {}", value);
      }
      _ => {}
    }
  }
}

#[allow(unused_assignments)]
pub extern "C" fn progress(mut c_handler: *mut c_void, progression: c_uchar) {
  if c_handler.is_null() {
    warn!("Null handler. Progression: {}%", progression);
    return;
  }

  let handler: Box<Handler> = unsafe { Box::from_raw(c_handler as *mut Handler) };
  if handler.job_id.is_none() {
    warn!("Null job id. Progression: {}%", progression);
    return;
  }

  publish_job_progression(
    handler.channel.clone(),
    handler.job_id.unwrap(),
    progression,
  )
  .map_err(|error| error!("Could not publish job progression: {:?}", error))
  .unwrap();

  c_handler = Box::into_raw(handler) as *mut c_void;
}

/************************
 *   Utility functions
 ************************/

pub(crate) fn get_library_file_path() -> String {
  std::env::var("WORKER_LIBRARY_FILE").unwrap_or_else(|_| "libworker.so".to_string())
}

pub(crate) unsafe fn get_library_function<'a, T>(
  library: &'a Library,
  func_name: &str,
) -> std::result::Result<libloading::Symbol<'a, T>, String> {
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

pub fn get_worker_parameters() -> Vec<WorkerParameter> {
  let mut parameters = vec![];
  match libloading::Library::new(get_library_file_path()) {
    Ok(worker_lib) => unsafe {
      // Retrieve number of parameters from the worker getter function
      let get_parameters_size_func: libloading::Symbol<GetParametersSizeFunc> =
        get_library_function(&worker_lib, constants::GET_PARAMETERS_SIZE_FUNCTION)
          .unwrap_or_else(|error| panic!(error));
      let parameters_size = get_parameters_size_func() as usize;

      // Allocate a C array to retrieve the worker parameters
      let worker_parameters =
        libc::malloc(std::mem::size_of::<CWorkerParameter>() * parameters_size)
          as *mut CWorkerParameter;

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

#[cfg(test)]
mod utils_test {

  use crate::{utils::get_parameter_value, Handler};
  use mcai_worker_sdk::job::Job;
  use std::ffi::{c_void, CString};

  #[test]
  pub fn test_c_get_parameter_value() {
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
    let parameters = job.get_parameters().ok();

    let handler = Handler {
      job_id: Some(job.job_id),
      parameters,
      channel: None,
    };

    let boxed_handler = Box::new(handler);
    let handler_ptr = Box::into_raw(boxed_handler) as *mut c_void;

    unsafe {
      let parameter_key = CString::new("path").unwrap();
      let parameter_id = parameter_key.as_ptr();

      println!(
        ">> get_parameter_value: {:?}, {:?}",
        handler_ptr, parameter_id
      );
      let c_value = get_parameter_value(handler_ptr, parameter_id);
      println!(
        "<< get_parameter_value: {:?}, {:?}",
        handler_ptr, parameter_id
      );
      assert!(!handler_ptr.is_null());
      assert!(!c_value.is_null());

      let value = get_c_string!(c_value);
      assert_eq!("/path/to/file".to_string(), value);
    }
  }

  #[test]
  pub fn test_c_get_unknown_parameter_value() {
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
    let parameters = job.get_parameters().ok();

    let handler = Handler {
      job_id: Some(job.job_id),
      parameters,
      channel: None,
    };

    let boxed_handler = Box::new(handler);
    let handler_ptr = Box::into_raw(boxed_handler) as *mut c_void;

    let parameter_key = CString::new("other_parameter").unwrap();

    let c_value = get_parameter_value(handler_ptr, parameter_key.as_ptr());
    assert!(!handler_ptr.is_null());
    assert!(c_value.is_null());

    unsafe {
      let value = get_c_string!(c_value);
      assert_eq!("".to_string(), value);
    }
  }

  #[test]
  pub fn test_c_get_parameter_value_with_null_ptr() {
    let null_handler = std::ptr::null_mut();
    let parameter_key = CString::new("path").unwrap();

    let c_value = get_parameter_value(null_handler, parameter_key.as_ptr());
    assert!(null_handler.is_null());
    assert!(c_value.is_null());

    unsafe {
      let value = get_c_string!(c_value);
      assert_eq!("".to_string(), value);
    }
  }
}
