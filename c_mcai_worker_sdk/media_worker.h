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
 * Audio / Video streams descriptors type
 */
enum StreamType {
    VIDEO,
    AUDIO,
    DATA
};

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

typedef const void* StreamDescriptor;
typedef const void* Filter;
typedef void* (*NewStreamDescriptorCallback)(unsigned int _index, StreamType _stream_type);
typedef void* (*NewFilterCallback)(const char* _filter_name, const char* _filter_label);

typedef void* (*AddDescriptorFilterCallback)(StreamDescriptor _stream_descriptor, Filter _filter);
typedef void* (*AddFilterParameterCallback)(Filter _filter, const char* _parameter_key, const char* _parameter_value);

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
 * @param handler                   Handler
 * @param parameters_value_getter   Get job parameter value callback
 * @param logger                    Rust Logger
 * @param format_context            Format context pointer
 * @param output_stream_descriptors Pointer of descriptors of the output streams
 */
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
void ending_process(Logger logger);

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
