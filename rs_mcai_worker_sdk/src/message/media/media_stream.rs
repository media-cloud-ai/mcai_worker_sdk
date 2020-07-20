use log::{info, trace};
use ringbuf::Consumer;
use stainless_ffmpeg::{
  audio_decoder::AudioDecoder,
  filter_graph::FilterGraph,  
};
use stainless_ffmpeg_sys::*;
use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::io::{Cursor, Error, ErrorKind, Result};
use std::mem;
use std::ptr::null_mut;
use std::str::from_utf8_unchecked;

unsafe fn to_string(data: *const i8) -> String {
  if data.is_null() {
    return "".to_string();
  }
  from_utf8_unchecked(CStr::from_ptr(data).to_bytes()).to_string()
}

macro_rules! check_result {
  ($condition: expr, $block: block) => {
    let errnum = $condition;
    if errnum < 0 {
      let mut data = [0i8; AV_ERROR_MAX_STRING_SIZE];
      av_strerror(errnum, data.as_mut_ptr(), AV_ERROR_MAX_STRING_SIZE as u64);
      $block;
      return Err(Error::new(
        ErrorKind::InvalidInput,
        to_string(data.as_ptr()),
      ));
    }
  };
  ($condition: expr) => {
    let errnum = $condition;
    if errnum < 0 {
      let mut data = [0i8; AV_ERROR_MAX_STRING_SIZE];
      av_strerror(errnum, data.as_mut_ptr(), AV_ERROR_MAX_STRING_SIZE as u64);
      return Err(Error::new(
        ErrorKind::InvalidInput,
        to_string(data.as_ptr()),
      ));
    }
  };
}

#[derive(Debug)]
pub struct MediaStream {
  pub format_context: *mut AVFormatContext,
  stream_info: bool,
  stream_ids: Vec<u8>,
  decoders: HashMap<u8, Decoder>,
  graph: Option<FilterGraph>,
}

#[derive(Debug)]
pub struct Decoder {
  codec: *mut AVCodec,
  context: *mut AVCodecContext,
  decoder: AudioDecoder,
}

unsafe extern "C" fn read_data(opaque: *mut c_void, raw_buffer: *mut u8, buf_size: i32) -> i32 {
  trace!("Read more data: {} bytes", buf_size);
  let consumer: &mut Consumer<u8> = &mut *(opaque as *mut Consumer<u8>);

  if consumer.is_empty() {
    return 0;
  }

  let vec = Vec::from_raw_parts(raw_buffer, buf_size as usize, buf_size as usize);

  let mut buffer = Cursor::new(vec);
  let size = consumer.write_into(&mut buffer, Some(buf_size as usize)).unwrap();

  mem::forget(buffer);
  size as i32
}

impl MediaStream {
  pub fn new(format: &str, consumer: Consumer<u8>) -> Result<Self> {
    unsafe {
      av_log_set_level(AV_LOG_ERROR);
      av_log_set_level(AV_LOG_QUIET);
    }

    let buffer_size = 2048;
    let mut format_context = unsafe { avformat_alloc_context() };

    unsafe {
      let buffer = av_malloc(buffer_size);

      let cformat = CString::new(format).unwrap();
      let av_input_format = av_find_input_format(cformat.as_ptr());
      info!("[FFMpeg] Open dynamic buffer");

      let writable_buffer = 0;
      let opaque = Box::new(consumer);

      let avio_context = avio_alloc_context(
        buffer as *mut u8,
        buffer_size as i32,
        writable_buffer,
        Box::into_raw(opaque) as *mut c_void,
        Some(read_data),
        None,
        None,
      );
      (*format_context).pb = avio_context;

      info!("[FFMpeg] Open Input");
      check_result!(avformat_open_input(
        &mut format_context,
        null_mut(),
        av_input_format,
        null_mut(),
      ));
    }

    info!("MediaStream created");

    Ok(MediaStream {
      decoders: HashMap::new(),
      format_context,
      stream_info: false,
      stream_ids: vec![],
      graph: None,
    })
  }

  pub fn stream_info(&self) -> Result<()> {
    info!("[FFMpeg] Find stream info");
    unsafe {
      check_result!(avformat_find_stream_info(self.format_context, null_mut()));
      Ok(())
    }
  }
}
