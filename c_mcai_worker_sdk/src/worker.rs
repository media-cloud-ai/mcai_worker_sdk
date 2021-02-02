use mcai_worker_sdk::prelude::*;

use crate::{
  constants,
  parameters::CWorkerParameters,
  process_return::ProcessReturn,
  types::{InitFunc, ProcessFunc},
  utils::*,
};
#[cfg(feature = "media")]
use crate::{
  media::{
    filters::{add_descriptor_filter, add_filter_parameter, new_filter, new_stream_descriptor},
    stream_descriptors::CStreamDescriptor,
  },
  types::{EndingProcessFunc, InitProcessFunc, ProcessFrameFunc},
};
use std::{ffi::c_void, os::raw::c_char};
#[cfg(feature = "media")]
use std::{
  mem::size_of,
  os::raw::c_uint,
  sync::{mpsc::Sender, Arc, Mutex},
};

#[repr(C)]
#[derive(Debug)]
pub struct CWorkerParameter {
  pub identifier: *const c_char,
  pub label: *const c_char,
  pub kind_size: usize,
  pub kind: *const *const c_char,
  pub required: i32,
}

#[derive(Clone, Debug, Default)]
pub struct CWorkerEvent {
  #[cfg(feature = "media")]
  result: Option<Arc<Mutex<Sender<ProcessResult>>>>,
}

impl MessageEvent<CWorkerParameters> for CWorkerEvent {
  fn get_name(&self) -> String {
    get_worker_function_string_value(constants::GET_NAME_FUNCTION)
  }

  fn get_short_description(&self) -> String {
    get_worker_function_string_value(constants::GET_SHORT_DESCRIPTION_FUNCTION)
  }

  fn get_description(&self) -> String {
    get_worker_function_string_value(constants::GET_DESCRIPTION_FUNCTION)
  }

  fn get_version(&self) -> Version {
    let version = get_worker_function_string_value(constants::GET_VERSION_FUNCTION);
    Version::parse(&version).unwrap_or_else(|_| {
      panic!(
        "unable to parse version {} (please use SemVer format)",
        version
      )
    })
  }

  fn init(&mut self) -> Result<()> {
    call_optional_worker_init()
  }

  #[cfg(feature = "media")]
  fn init_process(
    &mut self,
    parameters: CWorkerParameters,
    format_context: Arc<Mutex<FormatContext>>,
    result: Arc<Mutex<Sender<ProcessResult>>>,
  ) -> Result<Vec<StreamDescriptor>> {
    self.result = Some(result);
    call_worker_init_process(parameters, format_context)
  }

  #[cfg(feature = "media")]
  fn process_frame(
    &mut self,
    job_result: JobResult,
    stream_index: usize,
    process_frame: ProcessFrame,
  ) -> Result<ProcessResult> {
    call_worker_process_frame(job_result, stream_index, process_frame)
  }

  #[cfg(feature = "media")]
  fn ending_process(&mut self) -> Result<()> {
    let ending_process_result = call_worker_ending_process();

    if let Some(result) = &self.result {
      result
        .lock()
        .unwrap()
        .send(ProcessResult::end_of_process())
        .unwrap();
    }

    ending_process_result
  }

  fn process(
    &self,
    channel: Option<McaiChannel>,
    parameters: CWorkerParameters,
    job_result: JobResult,
  ) -> Result<JobResult> {
    debug!("Process job: {}", job_result.get_job_id());
    let process_return = call_worker_process(job_result.clone(), parameters, channel)?;
    debug!("Returned: {:?}", process_return);
    process_return.as_result(job_result)
  }
}

pub fn call_optional_worker_init() -> Result<()> {
  let library = get_library_file_path();
  debug!(
    "Call worker {} from library: {}",
    constants::INIT_FUNCTION,
    library
  );

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
) -> Result<Vec<StreamDescriptor>> {
  let library = get_library_file_path();
  debug!(
    "Call worker {} from library: {}",
    constants::INIT_PROCESS_FUNCTION,
    library
  );

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
    let format_context = format_context.lock().unwrap();
    let av_format_context_ptr = format_context.format_context;

    let c_stream_descriptors: &mut [*mut CStreamDescriptor; 256] = &mut [std::ptr::null_mut(); 256];
    let output_stream_descriptors_size_ptr = libc::malloc(size_of::<c_uint>()) as *mut c_uint;

    let return_code = init_process_func(
      handler_ptr as *mut c_void,
      new_stream_descriptor,
      new_filter,
      add_descriptor_filter,
      add_filter_parameter,
      logger,
      av_format_context_ptr as *mut c_void,
      &(c_stream_descriptors.as_mut_ptr() as *mut c_void),
      output_stream_descriptors_size_ptr,
    );

    if return_code != 0 {
      return Err(MessageError::RuntimeError(format!(
        "{:?} function returned error code: {:?}",
        constants::INIT_PROCESS_FUNCTION,
        return_code
      )));
    }

    let mut output_stream_descriptors_size = 0;
    if !output_stream_descriptors_size_ptr.is_null() {
      output_stream_descriptors_size = *output_stream_descriptors_size_ptr;
      libc::free(output_stream_descriptors_size_ptr as *mut c_void);
    }

    let mut output_stream_descriptors = vec![];

    for i in 0..output_stream_descriptors_size {
      let c_stream_descriptor_ptr = c_stream_descriptors[i as usize];

      if !c_stream_descriptor_ptr.is_null() {
        let c_stream_descriptor = Box::from_raw(c_stream_descriptor_ptr as *mut CStreamDescriptor);
        output_stream_descriptors.push(c_stream_descriptor.into());
      } else {
        break;
      }
    }
    Ok(output_stream_descriptors)
  }
}

#[cfg(feature = "media")]
pub fn call_worker_process_frame(
  job_result: JobResult,
  stream_index: usize,
  process_frame: ProcessFrame,
) -> Result<ProcessResult> {
  let library = get_library_file_path();
  debug!(
    "Call worker {} from library: {}",
    constants::PROCESS_FRAME_FUNCTION,
    library
  );

  let worker_lib = libloading::Library::new(library).map_err(|error| {
    MessageError::RuntimeError(format!(
      "Could not load worker dynamic library: {:?}",
      error
    ))
  })?;

  debug!("Loaded library!");

  unsafe {
    let handler = Handler {
      job_id: Some(job_result.get_job_id()),
      parameters: None,
      channel: None,
    };

    let handler_ptr = Box::into_raw(Box::new(handler));
    debug!("handler_ptr: {:?}", handler_ptr);

    let json_ptr = std::ptr::null();

    debug!("json_ptr: {:?}", json_ptr);

    let return_code = match process_frame {
      ProcessFrame::AudioVideo(frame) => {
        let process_frame_func: libloading::Symbol<ProcessFrameFunc> =
          get_library_function(&worker_lib, constants::PROCESS_FRAME_FUNCTION).map_err(
            |error| {
              MessageError::RuntimeError(format!(
                "Could not access {:?} function from worker library: {:?}",
                constants::PROCESS_FRAME_FUNCTION,
                error
              ))
            },
          )?;
        let av_frame_ptr = frame.frame;

        process_frame_func(
          handler_ptr as *mut c_void,
          get_parameter_value,
          logger,
          job_result.get_job_id() as u32,
          stream_index as u32,
          av_frame_ptr as *mut c_void,
          &json_ptr,
        )
      }
      ProcessFrame::EbuTtmlLive(_ebu_ttml_live) => {
        return Err(MessageError::NotImplemented());
      }
      ProcessFrame::Data(_) => {
        return Err(MessageError::NotImplemented());
      }
    };

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
  debug!(
    "Call worker {} from library: {}",
    constants::ENDING_PROCESS_FUNCTION,
    library
  );

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

pub fn call_worker_process(
  job_result: JobResult,
  parameters: CWorkerParameters,
  channel: Option<McaiChannel>,
) -> Result<ProcessReturn> {
  let library = get_library_file_path();
  debug!(
    "Call worker {} from library: {}",
    constants::PROCESS_FUNCTION,
    library
  );

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
      let message = if !message_ptr.is_null() {
        let from_c_string = get_c_string!(message_ptr);
        libc::free(message_ptr as *mut c_void);
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
