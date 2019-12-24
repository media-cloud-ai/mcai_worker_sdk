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
char* kind[1] = { (char*)"string" };
Parameter worker_parameters[1] = {
    {
        .identifier = (char*)"my_parameter",
        .label = (char*)"My parameter",
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

/**
 * Worker main process function
 * @param job                      Job parameters handler
 * @param parametersValueGetter    Get job parameter value callback
 * @param logger                   Rust Logger
 */
int process(
    JobHandle job_handle,
    GetParameterValueCallback parametersValueGetter,
    Logger logger,
    const char** message,
    const char*** output_paths
  ) {
    // Print message through the Rust Logger
    logger("debug", "Start C Worker process...");

    // Retrieve "path" job parameter value
    char* value = parametersValueGetter(job_handle, "path");

    // Check whether an error occurred parsing job parameters
    if(value == NULL) {
        const char* message_str = "Something went wrong...\0";
        size_t length = strlen(message_str) + 1;
        *message = (const char *)malloc(length);
        memcpy((void*)*message, message_str, length);
        return 1;
    }

    // Print value through the Rust Logger
    logger("debug", value);

    const char* message_str = "Everything worked well!\0";
    size_t length = strlen(message_str) + 1;
    *message = (const char *)malloc(length);
    memcpy((void*)*message, message_str, length);

    output_paths[0] = (const char **)malloc(sizeof(int) * 2);
    output_paths[0][0] = (const char *)malloc(20);
    output_paths[0][1] = 0;
    memcpy((void*)output_paths[0][0], "/path/out.mxf\0", 13);

    return 0;
}

#ifdef __cplusplus
}
#endif
