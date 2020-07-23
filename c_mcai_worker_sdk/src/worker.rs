use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uint, c_void};
#[cfg(not(feature = "media"))]
use std::os::raw::{c_int, c_uchar};
use std::ptr::null;

use libloading::Library;
use serde_json::Value;

use mcai_worker_sdk::{
  debug, error, info, trace, warn,
  worker::{Parameter, ParameterType},
  McaiChannel, MessageError, Result,
};
#[cfg(not(feature = "media"))]
use mcai_worker_sdk::{job::JobResult, publish_job_progression};
#[cfg(feature = "media")]
use mcai_worker_sdk::{FormatContext, Frame, ProcessResult};

use crate::constants;
use crate::parameters::CWorkerParameters;
#[cfg(not(feature = "media"))]
use crate::process_return::ProcessReturn;
#[cfg(feature = "media")]
use std::str::FromStr;
#[cfg(feature = "media")]
use std::sync::{Arc, Mutex};

macro_rules! get_c_string {
  ($name:expr) => {
    if $name.is_null() {
      "".to_string()
    } else {
      std::str::from_utf8_unchecked(CStr::from_ptr($name).to_bytes()).to_string()
    }
  };
}

#[repr(C)]
#[derive(Debug)]
pub struct Handler {
  pub job_id: Option<u64>,
  pub parameters: Option<CWorkerParameters>,
  pub channel: Option<McaiChannel>,
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
type InitFunc = unsafe fn(logger: LoggerCallback);
#[cfg(feature = "media")]
type InitProcessFunc = unsafe fn(
  handler: *mut c_void,
  callback: GetParameterValueCallback,
  logger: LoggerCallback,
  av_format_context: *mut c_void,
  output_stream_indexes: &*mut c_uint,
) -> c_uint;
#[cfg(feature = "media")]
type ProcessFrameFunc = unsafe fn(
  handler: *mut c_void,
  callback: GetParameterValueCallback,
  logger: LoggerCallback,
  stream_index: c_uint,
  frame: *mut c_void,
  output_message: &*const c_char,
) -> c_uint;
#[cfg(feature = "media")]
type EndingProcessFunc = unsafe fn(logger: LoggerCallback);
#[cfg(not(feature = "media"))]
type ProcessFunc = unsafe fn(
  handler: *mut c_void,
  callback: GetParameterValueCallback,
  progress: ProgressCallback,
  logger: LoggerCallback,
  output_message: &*const c_char,
  output_paths: &*mut *const c_char,
) -> c_int;
#[cfg(not(feature = "media"))]
type ProgressCallback = extern "C" fn(*mut c_void, c_uchar);

#[allow(unused_assignments)]
extern "C" fn get_parameter_value(
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

extern "C" fn logger(level: *const c_char, raw_value: *const c_char) {
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

#[cfg(not(feature = "media"))]
#[allow(unused_assignments)]
extern "C" fn progress(mut c_handler: *mut c_void, progression: c_uchar) {
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

pub fn call_optional_worker_init() -> Result<()> {
  let library = get_library_file_path();
  debug!("Call worker process from library: {}", library);

  let worker_lib = libloading::Library::new(library).map_err(|error| {
    MessageError::RuntimeError(format!(
      "Could not load worker dynamic library: {:?}",
      error
    ))
  })?;

  unsafe {
    if let Ok(init_func) =
      get_library_function::<libloading::Symbol<InitFunc>>(&worker_lib, constants::INIT_FUNCTION)
    {
      init_func(logger);
    }
  }

  Ok(())
}

#[cfg(feature = "media")]
pub fn call_worker_init_process(
  parameters: CWorkerParameters,
  format_context: Arc<Mutex<FormatContext>>,
) -> Result<Vec<usize>> {
  let library = get_library_file_path();
  debug!("Call worker process from library: {}", library);

  let worker_lib = libloading::Library::new(library).map_err(|error| {
    MessageError::RuntimeError(format!(
      "Could not load worker dynamic library: {:?}",
      error
    ))
  })?;

  unsafe {
    let init_process_func: libloading::Symbol<InitProcessFunc> =
      get_library_function(&worker_lib, constants::INIT_PROCESS_FUNCTION).map_err(|error| {
        MessageError::RuntimeError(format!(
          "Could not access {:?} function from worker library: {:?}",
          constants::INIT_PROCESS_FUNCTION,
          error
        ))
      })?;

    let handler = Handler {
      job_id: None,
      parameters: Some(parameters),
      channel: None,
    };

    let handler_ptr = Box::into_raw(Box::new(handler));
    let format_context_ptr = Box::into_raw(Box::new(format_context.lock().unwrap()));

    let output_stream_indexes_ptr = Vec::<c_uint>::new().as_mut_ptr();
    let return_code = init_process_func(
      handler_ptr as *mut c_void,
      get_parameter_value,
      logger,
      format_context_ptr as *mut c_void,
      &output_stream_indexes_ptr,
    );

    if return_code != 0 {
      return Err(MessageError::RuntimeError(format!(
        "{:?} function returned error code: {:?}",
        constants::INIT_PROCESS_FUNCTION,
        return_code
      )));
    }

    let mut output_streams = vec![];
    if !output_stream_indexes_ptr.is_null() {
      let mut offset = 0;
      loop {
        let value_ptr = output_stream_indexes_ptr.offset(offset);
        if value_ptr.is_null() {
          break;
        }
        output_streams.push((*value_ptr) as usize);
        offset += 1;
      }
    }

    Ok(output_streams)
  }
}

#[cfg(feature = "media")]
pub fn call_worker_process_frame(
  str_job_id: &str,
  stream_index: usize,
  frame: Frame,
) -> Result<ProcessResult> {
  let library = get_library_file_path();
  debug!("Call worker process from library: {}", library);

  let worker_lib = libloading::Library::new(library).map_err(|error| {
    MessageError::RuntimeError(format!(
      "Could not load worker dynamic library: {:?}",
      error
    ))
  })?;

  unsafe {
    let process_frame_func: libloading::Symbol<ProcessFrameFunc> =
      get_library_function(&worker_lib, constants::PROCESS_FRAME_FUNCTION).map_err(|error| {
        MessageError::RuntimeError(format!(
          "Could not access {:?} function from worker library: {:?}",
          constants::PROCESS_FRAME_FUNCTION,
          error
        ))
      })?;

    let job_id = u64::from_str(&str_job_id).ok();

    let handler = Handler {
      job_id,
      parameters: None,
      channel: None,
    };

    let handler_ptr = Box::into_raw(Box::new(handler));
    let frame_ptr = Box::into_raw(Box::new(frame));

    let json_ptr = std::ptr::null();

    let return_code = process_frame_func(
      handler_ptr as *mut c_void,
      get_parameter_value,
      logger,
      stream_index as u32,
      frame_ptr as *mut c_void,
      &json_ptr,
    );

    if return_code != 0 {
      return Err(MessageError::RuntimeError(format!(
        "{:?} function returned error code: {:?}",
        constants::PROCESS_FRAME_FUNCTION,
        return_code
      )));
    }

    let json = get_c_string!(json_ptr);
    libc::free(json_ptr as *mut libc::c_void);

    Ok(ProcessResult::new_json(&json))
  }
}

#[cfg(feature = "media")]
pub fn call_worker_ending_process() -> Result<()> {
  let library = get_library_file_path();
  debug!("Call worker process from library: {}", library);

  let worker_lib = libloading::Library::new(library).map_err(|error| {
    MessageError::RuntimeError(format!(
      "Could not load worker dynamic library: {:?}",
      error
    ))
  })?;

  unsafe {
    let process_func: libloading::Symbol<EndingProcessFunc> =
      get_library_function(&worker_lib, constants::ENDING_PROCESS_FUNCTION).map_err(|error| {
        MessageError::RuntimeError(format!(
          "Could not access {:?} function from worker library: {:?}",
          constants::ENDING_PROCESS_FUNCTION,
          error
        ))
      })?;

    process_func(logger);
  }
  Ok(())
}

#[cfg(not(feature = "media"))]
pub fn call_worker_process(
  job_result: JobResult,
  parameters: CWorkerParameters,
  channel: Option<McaiChannel>,
) -> Result<ProcessReturn> {
  let library = get_library_file_path();
  debug!("Call worker process from library: {}", library);

  let worker_lib = libloading::Library::new(library).map_err(|error| {
    MessageError::RuntimeError(format!(
      "Could not load worker dynamic library: {:?}",
      error
    ))
  })?;

  unsafe {
    let process_func: libloading::Symbol<ProcessFunc> =
      get_library_function(&worker_lib, constants::PROCESS_FUNCTION).map_err(|error| {
        MessageError::RuntimeError(format!(
          "Could not access {:?} function from worker library: {:?}",
          constants::PROCESS_FUNCTION,
          error
        ))
      })?;

    let handler = Handler {
      job_id: Some(job_result.get_job_id()),
      parameters: Some(parameters),
      channel,
    };

    let boxed_handler = Box::new(handler);
    let handler_ptr = Box::into_raw(boxed_handler);

    let message_ptr = std::ptr::null();

    let mut output_paths_ptr = vec![std::ptr::null()];
    let ptr = output_paths_ptr.as_mut_ptr();

    // Call C worker process function
    let return_code = process_func(
      handler_ptr as *mut c_void,
      get_parameter_value,
      progress,
      logger,
      &message_ptr,
      &ptr,
    );

    let mut output_paths = vec![];

    if return_code != 0 {

      let message =
        if !message_ptr.is_null() {
          let from_c_string = get_c_string!(message_ptr);
          libc::free(message_ptr as *mut libc::c_void);
          from_c_string
        } else {
          format!(
            "{:?} function returned error code: {:?}",
            constants::PROCESS_FUNCTION,
            return_code
          )
        };

      return Ok(ProcessReturn::new_error(&message));
    }

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

    Ok(ProcessReturn::new(return_code, &message).with_output_paths(output_paths))
  }
}


#[cfg(test)]
use mcai_worker_sdk::job::Job;

#[test]
#[cfg(not(feature = "media"))]
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
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters().unwrap();

  let returned_code = call_worker_process(job_result, parameters, None).unwrap();
  assert_eq!(returned_code.get_code(), 0);
  assert_eq!(returned_code.get_message(), "Everything worked well!");
  assert_eq!(
    returned_code.get_output_paths(),
    &vec!["/path/out.mxf".to_string()]
  );
}

#[test]
#[cfg(not(feature = "media"))]
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
  let job_result = JobResult::from(job.clone());
  let parameters = job.get_parameters().unwrap();

  let returned_code = call_worker_process(job_result, parameters, None).unwrap();
  assert_eq!(returned_code.get_code(), 1);
  assert_eq!(returned_code.get_message(), "Something went wrong...");
  assert!(returned_code.get_output_paths().is_empty());
}

#[test]
#[cfg(not(feature = "media"))]
pub fn test_c_progress_ptr() {
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

  progress(handler_ptr, 25);
  assert!(!handler_ptr.is_null());
}

#[test]
#[cfg(not(feature = "media"))]
pub fn test_c_progress_with_null_ptr() {
  let null_handler = std::ptr::null_mut();
  progress(null_handler, 50);
  assert!(null_handler.is_null());
}

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

    let c_value = get_parameter_value(handler_ptr, parameter_id);
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
