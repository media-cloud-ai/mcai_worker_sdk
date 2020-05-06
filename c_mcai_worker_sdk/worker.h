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
 * Job pointer
 */
typedef void* JobPointer;
/**
 * Channel pointer
 */
typedef void* ChannelPointer;
/**
 * Get job parameter value callback
 */
typedef char* (*GetParameterValueCallback)(JobPointer, const char*);
/**
 * Rust Logger
 */
typedef void* (*Logger)(const char*, const char*);
/**
 * Progress callback
 */
typedef void* (*ProgressCallback)(JobPointer, ChannelPointer, unsigned int, unsigned int);

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
 * Worker main process function
 * @param job_pointer              Job pointer
 * @param channel_pointer          Channel pointer
 * @param parameters_value_getter  Get job parameter value callback
 * @param progress_callback        Progress callback
 * @param logger                   Rust Logger
 * @param message                  Output message pointer
 * @param output_paths             Output paths pointer
 */
int process(
    JobPointer job_pointer,
    ChannelPointer channel_pointer,
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
