use std::os::raw::{c_char, c_int, c_uchar, c_uint, c_void};

#[cfg(feature = "media")]
use crate::media::filters::{
  AddDescriptorFilterCallback, AddFilterParameterCallback, NewFilterCallback,
  NewStreamDescriptorCallback,
};
use crate::worker::WorkerParameter;

pub(crate) type GetStringFunc = unsafe fn() -> *const c_char;
pub(crate) type GetParametersSizeFunc = unsafe fn() -> c_uint;
pub(crate) type GetParametersFunc = unsafe fn(parameters: *mut WorkerParameter);

pub(crate) type GetParameterValueCallback =
  extern "C" fn(*mut c_void, *const c_char) -> *const c_char;
pub(crate) type LoggerCallback = extern "C" fn(*const c_char, *const c_char);
pub(crate) type InitFunc = unsafe fn(logger: LoggerCallback);

#[cfg(feature = "media")]
pub(crate) type InitProcessFunc = unsafe fn(
  handler: *mut c_void,
  new_stream_descriptor_callback: NewStreamDescriptorCallback,
  new_filter_callback: NewFilterCallback,
  add_descriptor_filter_callback: AddDescriptorFilterCallback,
  add_filter_parameter_callback: AddFilterParameterCallback,
  logger: LoggerCallback,
  av_format_context: *mut c_void,
  output_stream_descriptors: &*mut c_void,
  output_stream_descriptors_size: *mut c_uint,
) -> c_uint;

#[cfg(feature = "media")]
pub(crate) type ProcessFrameFunc = unsafe fn(
  handler: *mut c_void,
  callback: GetParameterValueCallback,
  logger: LoggerCallback,
  job_id: c_uint,
  stream_index: c_uint,
  frame: *mut c_void,
  output_message: &*const c_char,
) -> c_uint;

#[cfg(feature = "media")]
pub(crate) type EndingProcessFunc = unsafe fn(logger: LoggerCallback);

pub(crate) type ProcessFunc = unsafe fn(
  handler: *mut c_void,
  callback: GetParameterValueCallback,
  progress: ProgressCallback,
  logger: LoggerCallback,
  output_message: &*const c_char,
  output_paths: &*mut *const c_char,
) -> c_int;

pub(crate) type ProgressCallback = extern "C" fn(*mut c_void, c_uchar);
