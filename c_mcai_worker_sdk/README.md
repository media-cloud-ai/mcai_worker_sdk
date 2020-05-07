# C/C++ binding for Rust AMQP Worker
Based on [rs_amqp_worker](https://github.com/media-cloud-ai/rs_amqp_worker).

[![Build Status](https://travis-ci.org/media-cloud-ai/c_amqp_worker.svg?branch=master)](https://travis-ci.org/media-cloud-ai/c_amqp_worker)
[![Coverage Status](https://coveralls.io/repos/github/media-cloud-ai/c_amqp_worker/badge.svg?branch=master)](https://coveralls.io/github/media-cloud-ai/c_amqp_worker?branch=master)

## Build
To build the rust application
```bash
cargo build
```

To build the provided worker example
```bash
gcc -c -Wall -Werror -fpic worker.cpp
gcc -shared -o libworker.so worker.o
```

## Test
To run the unit tests, you must build the provided worker example (see the Build section above).
```bash
cargo test
```
## Usage

This worker uses Rust FFI to load a C/C++ Shared Object library, and to execute it. The C/C++ worker must implement some functions to be correctly bound:

 * `char* get_name()`: to retrieve the worker name
 * `char* get_short_description()`: to retrieve a short description of the worker
 * `char* get_description()`: to describe the worker purpose
 * `char* get_version()`: to retrieve the worker version
 * `unsigned int get_parameters_size()`: to return the number of parameter, before calling...
 * `void get_parameters(Parameter*) `: to fill the allocated pointer with the worker parameters
 * `int process(void*, char* (*)(void*, const char*), void* ()(void*, unsigned char), void* (*)(const char*, const char*), const char**, const char***)`: to execute the worker process

For more details, see the provided [worker.cpp](worker.cpp) example.

Set the `WORKER_LIBRARY_FILE` environment variable to specify the path of your C/C++ worker shared-library. Otherwise, the `libworker.so` file will be loaded by default.
