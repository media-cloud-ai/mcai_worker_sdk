#include <stdio.h>
#include <string.h>
#include "parameter.h"

char* get_name() {
	return "my_c_worker";
}

char* get_short_description() {
	return "My C Worker";
}

char* get_description() {
	return "This is my long description \n\
over multilines";
}

char* get_version() {
	return "0.1.0";
}

char* kind[1] = { "string" };
Parameter worker_parameters[1] = {
    {
        .identifier = "my_parameter",
        .label = "My parameter",
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

int process(unsigned int argc, char **argv) {
    return 0;
}
