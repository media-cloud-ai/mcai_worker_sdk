import json
import logging
import os

def get_name():
    return "My python Media Worker"


def get_short_description():
    return "My python Media Worker"


def get_description():
    return """This is my long description
	over multilines
	"""


def get_version():
    return "0.0.3"


def get_parameters():
    return [
        {
            "identifier": "source_path",
            "label": "My parameter",
            "kind": ["string"],
            "required": True,
        },
        {
            "identifier": "destination_path",
            "label": "My array parameter",
            "kind": ["string"],
            "required": False,
        },
        {
            "identifier": "requirements",
            "label": "Requirements",
            "kind": ["requirement"],
            "required": False,
        }
    ]


def init():
    '''
    Optional worker initialization function.
    '''

    print("Initialise Python worker...")

    log_level = os.environ.get('RUST_LOG', 'warning').upper()
    logging.basicConfig(format='[%(levelname)s] %(message)s', level=log_level)


def init_process(stream_handler, format_context, parameters):
    '''
    Function called before the media process (the "media" feature must be activated).
    '''
    logging.info("Initialise the media process...")
    logging.debug("Number of streams: %d", format_context.nb_streams)
    logging.debug("Message parameters: %s", parameters)

    # Here audio/video filters can be set to be applied on the worker input frames, using a simple python dict as follow.
    # Check the FFmpeg documentation to have more details on filters usage: https://ffmpeg.org/ffmpeg-filters.html
    video_filters = [
        {
            "name": "crop",
            "label": "crop_filter",
            "parameters": {
               "out_w": "300",
               "out_h": "200",
               "x": "50",
               "y": "50"
            }
        }
    ]

    audio_filters = [
        {
            "name": "aformat",
            "parameters": {
                "sample_rates": "16000",
                "channel_layouts": "mono",
                "sample_fmts": "s16"
            }
        }
    ]

    video_stream = stream_handler.new_video_stream(0, video_filters)
    audio_stream = stream_handler.new_audio_stream(1, audio_filters)

    # returns a list of description of the streams to be processed
    return [
        video_stream,
        audio_stream
    ]


def process_frame(job_id, stream_index, frame):
    '''
    Process media frame (the "media" feature must be activated).
    '''
    data_length = 0
    for plane in range(0, len(frame.data)):
        data_length = data_length + len(frame.data[plane])

    if frame.width != 0 and frame.height != 0:
        logging.info(f"Job: {job_id} - Process video stream {stream_index} frame - PTS: {frame.pts}, image size: {frame.width}x{frame.height}, data length: {data_length}")
    else:
        logging.info(f"Job: {job_id} - Process audio stream {stream_index} frame - PTS: {frame.pts}, sample_rate: {frame.sample_rate}Hz, channels: {frame.channels}, nb_samples: {frame.nb_samples}, data length: {data_length}")

    # returns the process result as a JSON object (this is fully customisable)
    return { "status": "success" }


def ending_process():
    '''
    Function called at the end of the media process (the "media" feature must be activated).
    '''
    logging.info("Ending Python worker process...")
