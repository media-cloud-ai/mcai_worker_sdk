#include <stdlib.h>
#include <stdio.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Worker parameter type
 */
typedef struct Parameter {
    char* identifier;
    char* label;
    unsigned int kind_size;
    char** kind;
    int required;
} Parameter;

/**
 * Job & channel handler
 */
typedef void* Handler;

/**
 * Get job parameter value callback
 * @param _handler          the job & channel handler
 * @param _parameter_key    the name of the parameter to get
 * @return the parameter value
 * @note   the returned pointer must be freed by user.
 */
typedef char* (*GetParameterValueCallback)(Handler _handler, const char* _parameter_key);

/**
 * Rust Logger
 * @param _level      the log level: 'trace', 'debug', 'info', 'warn' or 'error'
 * @param _message    the message to log
 */
typedef void* (*Logger)(const char* _level, const char* _message);

/**
 * Progress callback
 * @param _handler                   the job & channel handler
 * @param _progression_percentage    the progression percentage (between 0 and 100)
 */
typedef void* (*ProgressCallback)(Handler _handler, unsigned char _progression_percentage);

/**
 * Get worker name
 */
char* get_name();

/**
 * Get worker short description
 */
char* get_short_description();

/**
 * Get worker long description
 */
char* get_description();

/**
 * Get worker version
 */
char* get_version();

/**
 * Get number of worker parameters
 */
unsigned int get_parameters_size();

/**
 * Retrieve worker parameters
 * @param parameters    Output parameters array pointer
 */
void get_parameters(Parameter* parameters);

/**
 * Initialize worker
 * (This fonction is optional)
 * @param logger  Rust Logger
 */
void init(Logger logger);

/**
 * Initialize worker media process
 * (the "media" feature must be enabled)
 * @param handler                  Handler
 * @param parameters_value_getter  Get job parameter value callback
 * @param logger                   Rust Logger
 * @param format_context           Format context pointer
 * @param output_stream_indexes    Pointer of indexes of the output streams
 */
int init_process(
    Handler handler,
    GetParameterValueCallback parameters_value_getter,
    Logger logger,
    void* format_context,
    unsigned int** output_stream_indexes,
    unsigned int* output_stream_indexes_size
  );

/**
 * Process the media frame
 * (the "media" feature must be enabled)
 * @param handler                  Handler
 * @param parameters_value_getter  Get job parameter value callback
 * @param logger                   Rust Logger
 * @param stream_index             Frame stream index
 * @param frame                    Frame pointer
 * @param message                  Output message pointer
 */
int process_frame(
    Handler handler,
    GetParameterValueCallback parameters_value_getter,
    Logger logger,
    const unsigned int job_id,
    const unsigned int stream_index,
    void* frame,
    const char** message
  );

/**
 * End the media process
 * (the "media" feature must be enabled)
 * @param logger  Rust Logger
 */
void endingProcess(Logger logger);

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
    GetParameterValueCallback parameters_value_getter,
    ProgressCallback progress_callback,
    Logger logger,
    const char** message,
    const char*** output_paths
  );

/**
 * Set the C string to the pointer
 * @param message  Pointer on the const char*
 * @param value    c string with 0 ending
 */
void set_str_on_ptr(const char** message, const char* value) {
    size_t length = strlen(value) + 1;
    *message = (const char *)malloc(length);
    memcpy((void*)*message, value, length);
}

#ifdef __cplusplus
}
#endif
