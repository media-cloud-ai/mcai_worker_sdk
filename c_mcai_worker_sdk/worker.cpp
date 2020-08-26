
#include "worker.h"
#include <libavformat/avformat.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Get worker name
 */
char* get_name() {
	return (char*)"my_c_worker";
}

/**
 * Get worker short description
 */
char* get_short_description() {
	return (char*)"My C Worker";
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
char* kind[3] = { (char*)"string" };
Parameter worker_parameters[3] = {
    {
        .identifier = (char*)"my_parameter",
        .label = (char*)"My parameter",
        .kind_size = 1,
        .kind = kind,
        .required = 0
    },
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
    GetParameterValueCallback parameters_value_getter,
    Logger logger,
    void* format_context,
    unsigned int** output_stream_indexes,
    unsigned int* output_stream_indexes_size
  ) {
    logger("debug", "Initialize C Worker media process...");

    // Cast to FFmpeg AVFormatContext pointer
    AVFormatContext* av_format_context = (AVFormatContext*)format_context;

    // Get nb streams
    const unsigned int nb_streams = av_format_context->nb_streams;
    const size_t length = sizeof(unsigned int) * nb_streams;
    *output_stream_indexes_size = nb_streams;

    // Return stream indexes
    unsigned int streams[nb_streams];
    for (unsigned int i = 0; i < nb_streams; ++i) {
        streams[i] = i;
    }
    memcpy(*output_stream_indexes, &streams, length);
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

/**
 * Worker main process function
 * @param handler                  Handler
 * @param parameters_value_getter  Get job parameter value callback
 * @param progress_callback        Progress callback
 * @param logger                   Rust Logger
 * @param message                  Output message pointer
 * @param output_paths             Output paths pointer
 */
int process(
    Handler handler,
    GetParameterValueCallback parametersValueGetter,
    ProgressCallback progressCallback,
    Logger logger,
    const char** message,
    const char*** output_paths
  ) {
    // Print message through the Rust Logger
    logger("debug", "Start C Worker process...");

    // Notify with progression
    progressCallback(handler, 0);

    // Retrieve "path" job parameter value
    char* value = parametersValueGetter(handler, "path");

    // Check whether an error occurred parsing job parameters
    if(value == NULL) {
        set_str_on_ptr(message, "Something went wrong...\0");
        return 1;
    }

    // Notify with progression
    progressCallback(handler, 33);

    // Print value through the Rust Logger
    logger("debug", value);

    // Free the parameter value pointer
    free(value);

    set_str_on_ptr(message, "Everything worked well!\0");

    output_paths[0] = (const char **)malloc(sizeof(int) * 2);
    set_str_on_ptr(&output_paths[0][0], "/path/out.mxf\0");
    output_paths[0][1] = 0;

    // Notify with progression
    progressCallback(handler, 100);

    return 0;
}

#ifdef __cplusplus
}
#endif
