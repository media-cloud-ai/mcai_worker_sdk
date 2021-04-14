use mcai_worker_sdk::message::media::source::Source;
use stainless_ffmpeg::{format_context::FormatContext, prelude::*, tools::rational::Rational};
use std::sync::{Arc, Mutex};

#[test]
pub fn test_media_source_seek() {
  let file_path = "./test_gop.mxf";
  let nb_frames = 50;

  super::ffmpeg::create_xdcam_sample_file(file_path, nb_frames).unwrap();

  let mut format_context = FormatContext::new(file_path).unwrap();
  format_context.open_input().unwrap();

  let time_base = Source::get_stream_time_base(0, &format_context);
  assert_eq!(Rational { num: 1, den: 25 }, time_base);

  let format_context_ref = Arc::new(Mutex::new(format_context));

  let packet = format_context_ref.lock().unwrap().next_packet().unwrap();
  let pts = unsafe { (*packet.packet).pts };
  assert_eq!(0, pts);

  let frame_index = 7;
  let milliseconds = Source::get_milliseconds_from_pts(frame_index, &time_base);
  assert_eq!(280, milliseconds);

  let result = Source::seek_in_stream_at(
    0,
    milliseconds as i64,
    format_context_ref.clone(),
    AVSEEK_FLAG_ANY | AVSEEK_FLAG_FRAME,
  );
  assert!(result.is_ok());

  let packet = format_context_ref.lock().unwrap().next_packet().unwrap();
  let pts = unsafe { (*packet.packet).pts };
  assert_eq!(7, pts);

  let frame_index = 9;
  let milliseconds = Source::get_milliseconds_from_pts(frame_index, &time_base);
  assert_eq!(360, milliseconds);

  let result = Source::seek_in_stream_at(
    0,
    milliseconds as i64,
    format_context_ref.clone(),
    AVSEEK_FLAG_BACKWARD,
  );
  assert!(result.is_ok());

  let packet = format_context_ref.lock().unwrap().next_packet().unwrap();
  let pts = unsafe { (*packet.packet).pts };
  assert_eq!(0, pts);

  std::fs::remove_file(file_path).unwrap();
}
