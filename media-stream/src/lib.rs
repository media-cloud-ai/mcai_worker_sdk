use std::collections::HashMap;
use std::ffi::{c_void, CStr, CString};
use std::io::{Cursor, Error, ErrorKind, Result};
use std::mem;
use std::ptr::null_mut;
use std::str::from_utf8_unchecked;

use log::{debug, error, info, trace, warn};
use ringbuf::Consumer;
use stainless_ffmpeg::{
  audio_decoder::AudioDecoder, filter_graph::FilterGraph, format_context::FormatContext,
  frame::Frame, order, order::filter_output::FilterOutput, order::parameters::ParameterValue,
  video_decoder::VideoDecoder,
};
use stainless_ffmpeg_sys::*;

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
  format_context: *mut AVFormatContext,
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
  let size = consumer
    .write_into(&mut buffer, Some(buf_size as usize))
    .unwrap();

  mem::forget(buffer);
  size as i32
}

impl MediaStream {
  pub fn new(source_url: &str) -> Result<Self> {
    unsafe {
      av_log_set_level(AV_LOG_ERROR);
      // av_log_set_level(AV_LOG_QUIET);
    }

    let mut context =
      FormatContext::new(source_url).map_err(|e| Error::new(ErrorKind::Other, e))?;
    context
      .open_input()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let video_decoder = VideoDecoder::new("h264".to_string(), &context, 0)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let mut graph = FilterGraph::new().map_err(|e| Error::new(ErrorKind::Other, e))?;
    graph
      .add_input_from_video_decoder("video_input", &video_decoder)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    graph
      .add_video_output("video_output")
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let parameters: HashMap<String, ParameterValue> = [("pix_fmts", "rgb24")]
      .iter()
      .cloned()
      .map(|(key, value)| (key.to_string(), ParameterValue::String(value.to_string())))
      .collect();

    let filter_definition = order::filter::Filter {
      name: "format".to_string(),
      label: Some("format_filter".to_string()),
      parameters,
      inputs: None,
      outputs: Some(vec![FilterOutput {
        stream_label: "video_output".to_string(),
      }]),
    };

    let filter = graph
      .add_filter(&filter_definition)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    graph
      .connect_input("video_input", 0, &filter, 0)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;
    graph
      .connect_output(&filter, 0, "video_output", 0)
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    let _result = graph
      .validate()
      .map_err(|e| Error::new(ErrorKind::Other, e))?;

    info!("MediaStream created");

    let mut stream_ids = vec![];
    for i in 0..context.get_nb_streams() as u8 {
      stream_ids.push(i);
    }

    Ok(MediaStream {
      decoders: HashMap::new(),
      format_context: context.format_context,
      stream_info: false,
      stream_ids,
      graph: None,
    })
  }

  pub fn new_stream(format: &str, consumer: Consumer<u8>, stream_ids: &[u8]) -> Result<Self> {
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
      stream_ids: stream_ids.to_vec(),
      graph: None,
    })
  }

  pub fn read_packet(&mut self) -> Result<Option<*mut AVPacket>> {
    if !self.stream_info {
      info!("[FFMpeg] Find stream info");
      unsafe {
        check_result!(avformat_find_stream_info(self.format_context, null_mut()));

        let stream_ids = self.stream_ids.clone();

        for stream_id in &stream_ids {
          let stream = (*self.format_context).streams.offset(*stream_id as isize);
          let source_codec = (**stream).codec;

          let refcounted_frames = CString::new("refcounted_frames").unwrap();
          av_opt_set_int(
            source_codec as *mut c_void,
            refcounted_frames.as_ptr(),
            1,
            0,
          );

          let codec = avcodec_find_decoder((*source_codec).codec_id);
          let context = avcodec_alloc_context3(codec);

          check_result!(avcodec_parameters_to_context(context, (**stream).codecpar));
          check_result!(avcodec_open2(context, codec, null_mut()));

          let audio_decoder = AudioDecoder {
            identifier: "audio_source_1".to_string(),
            stream_index: 1,
            codec_context: context,
          };
          self.create_graph(&audio_decoder);

          self.decoders.insert(
            *stream_id,
            Decoder {
              codec,
              context,
              decoder: audio_decoder,
            },
          );
        }
      }
      self.stream_info = true;
    }

    if self.stream_info {
      unsafe {
        let mut packet = av_packet_alloc();
        av_init_packet(packet);
        (*packet).data = null_mut();
        (*packet).size = 0;
        trace!("[FFMpeg] Read frame");

        check_result!(av_read_frame(self.format_context, packet));
        debug!(
          "[FFMpeg] Got a packet for stream ID {}",
          (*packet).stream_index
        );

        return Ok(Some(packet));
      }
    }

    Ok(None)
  }

  pub fn free_packet(&mut self, packet: Option<*mut AVPacket>) {
    if let Some(packet) = packet {
      if packet.is_null() {
        return;
      }
      trace!("[FFmpeg] Free packet");
      unsafe {
        av_free_packet(packet);
      }
    }
  }

  pub fn decode(&self, packet: &Option<*mut AVPacket>) -> Result<Option<*mut AVFrame>> {
    if packet.is_none() {
      return Ok(None);
    }

    let packet = packet.unwrap();
    unsafe {
      let stream_index = (*packet).stream_index as u8;

      if !self.stream_ids.contains(&stream_index) {
        return Ok(None);
      }
      debug!("[FFmpeg] New packet for stream {}", stream_index);

      if let Some(decoder) = self.decoders.get(&stream_index) {
        trace!("[FFmpeg] Send packet to decoder");
        check_result!(avcodec_send_packet(decoder.context, packet));

        let av_frame = av_frame_alloc();
        check_result!(avcodec_receive_frame(decoder.context, av_frame));
        trace!("[FFmpeg] Got a frame for stream {}", stream_index);

        let frame = Frame {
          frame: av_frame,
          name: Some("audio_source_1".to_string()),
          index: 1,
        };

        let av_frame = if let Some(graph) = &self.graph {
          trace!("[FFmpeg] Process graph");
          if let Ok((audio_frames, _video_frames)) = graph.process(&[frame], &[]) {
            trace!("[FFmpeg] Output graph count {} frames", audio_frames.len());
            let frame = audio_frames.first().unwrap();
            av_frame_clone((*frame).frame)
          } else {
            av_frame
          }
        } else {
          av_frame
        };

        return Ok(Some(av_frame));
      }
    }

    Ok(None)
  }

  pub fn free_frame(&mut self, frame: Option<*mut AVFrame>) {
    if let Some(frame) = &frame {
      error!("Free frame {:?} {:?}", frame, *frame);
      if frame.is_null() {
        return;
      }
      unsafe {
        av_frame_free((*frame) as *mut *mut AVFrame);
      }
    }
  }

  fn create_graph(&mut self, audio_decoder: &AudioDecoder) {
    if self.graph.is_some() {
      warn!("Try to create a graph a second time, skip it !");
      return;
    }

    let mut graph = FilterGraph::new().unwrap();

    graph
      .add_input_from_audio_decoder("audio_input", &audio_decoder)
      .unwrap();
    graph.add_audio_output("audio_output").unwrap();

    let parameters: HashMap<String, ParameterValue> = [
      ("channel_layouts", "mono"),
      ("sample_fmts", "s16"),
      ("sample_rates", "16000"),
    ]
    .iter()
    .cloned()
    .map(|(key, value)| (key.to_string(), ParameterValue::String(value.to_string())))
    .collect();

    let filter_definition = order::filter::Filter {
      name: "aformat".to_string(),
      label: Some("aformat_filter".to_string()),
      parameters,
      inputs: None,
      outputs: Some(vec![FilterOutput {
        stream_label: "audio_output".to_string(),
      }]),
    };

    let filter = graph.add_filter(&filter_definition).unwrap();
    graph.connect_input("audio_input", 0, &filter, 0).unwrap();
    graph.connect_output(&filter, 0, "audio_output", 0).unwrap();

    graph.validate().unwrap();

    self.graph = Some(graph);
  }
}
