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

int process(unsigned int argc, char **argv) {
    for(unsigned int i = 0; i < argc; i++) {
        printf("Argument %d: %s\n", i, argv[i]);
    }
    if(argc != 3) {
        return 1;
    }
    return 0;
}

#ifdef __cplusplus
}
#endif
