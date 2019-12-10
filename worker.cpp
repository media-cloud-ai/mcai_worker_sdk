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
typedef void* JobParameters;
/**
 * Get job parameter value callback
 */
typedef char* (*GetParameterValueCallback)(JobParameters, const char*);
/**
 * Rust logger callback
 */
typedef void* (*Logger)(const char*);
/**
 * Check error callback
 */
typedef int* (*CheckError)();


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
 * @param checkError               Check error callback
 * @param logger                   Rust logger callback
 */
int process(JobParameters job, GetParameterValueCallback parametersValueGetter, CheckError checkError, Logger logger) {
    // Print message through the Rust internal logger
    logger("Start C Worker process...");

    // Retrieve "path" job parameter value
    char* value = parametersValueGetter(job, "path");

    // Check whether an error occurred parsing job parameters
    if(checkError() != 0) {
        return 1;
    }

    // Print value through the Rust internal logger
    logger(value);
    return 0;
}

#ifdef __cplusplus
}
#endif
