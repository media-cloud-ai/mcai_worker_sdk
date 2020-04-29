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
 * Job parameters handler
 */
typedef void* JobHandle;
/**
 * Get job parameter value callback
 */
typedef char* (*GetParameterValueCallback)(JobHandle, const char*);
/**
 * Rust Logger
 */
typedef void* (*Logger)(const char*, const char*);

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
 * @param job_handle               Job parameters handler
 * @param parameters_value_getter  Get job parameter value callback
 * @param logger                   Rust Logger
 */
int process(
    JobHandle job_handle,
    GetParameterValueCallback parameters_value_getter,
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
