
#include "media_worker.h"
#include <libavformat/avformat.h>
#include <libavutil/avutil.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Get worker name
 */
char* get_name() {
	return (char*)"my_c_media_worker";
}

/**
 * Get worker short description
 */
char* get_short_description() {
	return (char*)"My C Media Worker";
}

/**
 * Get worker long description
 */
char* get_description() {
	return (char*)"This is my long description \n\
over multilines";
}

/**
 * Get worker version
 */
char* get_version() {
	return (char*)"0.1.0";
}

// Example of worker parameters
char* kind[1] = { (char*)"string" };
Parameter worker_parameters[2] = {
    {
        .identifier = (char*)"source_path",
        .label = (char*)"Source path",
        .kind_size = 1,
        .kind = kind,
        .required = 1
    },
    {
        .identifier = (char*)"destination_path",
        .label = (char*)"Destination path",
        .kind_size = 1,
        .kind = kind,
        .required = 1
    }
};

/**
 * Get number of worker parameters
 */
unsigned int get_parameters_size() {
    return sizeof(worker_parameters) / sizeof(Parameter);
}

/**
 * Retrieve worker parameters
 * @param parameters    Output parameters array pointer
 */
void get_parameters(Parameter* parameters) {
    memcpy(parameters, worker_parameters, sizeof(worker_parameters));
}

void init(Logger logger) {
    // Print message through the Rust Logger
    logger("debug", "Init C Worker...");
}

int init_process(
    Handler handler,
    NewStreamDescriptorCallback new_stream_descriptor_callback,
    NewFilterCallback new_filter_callback,
    AddDescriptorFilterCallback add_descriptor_filter_callback,
    AddFilterParameterCallback add_filter_parameter_callback,
    Logger logger,
    void* format_context,
    void** output_stream_descriptors,
    unsigned int* output_stream_descriptors_size
  ) {
    logger("debug", "Initialize C Worker media process...");

    // Cast to FFmpeg AVFormatContext pointer
    AVFormatContext* av_format_context = (AVFormatContext*)format_context;

    // Get nb streams
    const unsigned int nb_streams = av_format_context->nb_streams;
    const size_t length = sizeof(StreamDescriptor) * nb_streams;
    *output_stream_descriptors_size = nb_streams;

    // Return stream descriptors
    const void* stream_descriptors[nb_streams];

    for (unsigned int i = 0; i < nb_streams; ++i) {
        switch(av_format_context->streams[i]->codecpar->codec_type) {
            case AVMEDIA_TYPE_AUDIO: {
                logger("debug", "New audio stream descriptor...");
                const void* descriptor = new_stream_descriptor_callback(i, AUDIO);

                logger("debug", "New filter...");
                const void* filter = new_filter_callback("aformat", "aformat_filter");
                logger("debug", "Set parameters...");
                add_filter_parameter_callback(filter, "sample_rates", "16000");
                add_filter_parameter_callback(filter, "sample_fmts", "s32");
                add_filter_parameter_callback(filter, "channel_layouts", "mono");

                logger("debug", "Set filter to descriptor...");
                add_descriptor_filter_callback(descriptor, filter);

                stream_descriptors[i] = descriptor;
                break;
            }
            case AVMEDIA_TYPE_VIDEO: {
                logger("debug", "New video stream descriptor...");
                // do the same with video stream descriptor...
                break;
            }
            case AVMEDIA_TYPE_SUBTITLE:
            case AVMEDIA_TYPE_DATA: {
                logger("debug", "New data stream descriptor...");
                const void* descriptor = new_stream_descriptor_callback(i, DATA);
                stream_descriptors[i] = descriptor;
                break;
            }
            default:
                continue;
        }
    }

    memcpy(*output_stream_descriptors, &stream_descriptors, length);
    return 0;
}

int process_frame(
    Handler handler,
    GetParameterValueCallback parameters_value_getter,
    Logger logger,
    const unsigned int job_id,
    const unsigned int stream_index,
    void* frame,
    const char** message
  ) {
    // Cast to FFmpeg AVFrame pointer
    AVFrame* av_frame = (AVFrame*) frame;

    // Log process details
    char* info_message = (char*)malloc(256);
    if(av_frame->width != 0 && av_frame->height != 0) {
        sprintf(info_message, "Job: %d - Process video stream %d frame - PTS: %ld, image size: %dx%d, data: %p",
            job_id, stream_index, av_frame->pts, av_frame->width, av_frame->height, av_frame->data);
    } else {
        sprintf(info_message, "Job: %d - Process audio stream %d frame - PTS: %ld, sample_rate: %dHz, channels: %d, nb_samples: %d, data: %p",
            job_id, stream_index, av_frame->pts, av_frame->sample_rate, av_frame->channels, av_frame->nb_samples, av_frame->data);
    }
    logger("debug", info_message);
    free(info_message);

    // Return process result as JSON
    char* json_result = (char*)malloc(256);
    sprintf(json_result, "{\"job_id\": %d, \"pts\": %ld, \"result\":\"OK\"}", job_id, av_frame->pts);
    set_str_on_ptr(message, json_result);
    free(json_result);

    return 0;
}

void ending_process(Logger logger) {
    // Print message through the Rust Logger
    logger("debug", "Ending C Worker media process...");
}

#ifdef __cplusplus
}
#endif
