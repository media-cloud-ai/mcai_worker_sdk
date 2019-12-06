#include <stdio.h>
#include <string.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Parameter {
    char* identifier;
    char* label;
    unsigned int kind_size;
    char** kind;
    int required;
} Parameter;

typedef void* JobParameters;
typedef char* (*GetParameterValueCallback)(JobParameters, const char*);
typedef void* (*Logger)(const char*);

char* get_name() {
	return (char*)"my_c_worker";
}

char* get_short_description() {
	return (char*)"My C Worker";
}

char* get_description() {
	return (char*)"This is my long description \n\
over multilines";
}

char* get_version() {
	return (char*)"0.1.0";
}

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

unsigned int get_parameters_size() {
    return sizeof(worker_parameters) / sizeof(Parameter);
}

void get_parameters(Parameter* parameters) {
    memcpy(parameters, worker_parameters, sizeof(worker_parameters));
}

int process(JobParameters job, GetParameterValueCallback parametersValueGetter, Logger logger) {
    logger("Start C Worker process...");
    char* value = parametersValueGetter(job, "path");
    logger(value);
    return 0;
}

#ifdef __cplusplus
}
#endif
