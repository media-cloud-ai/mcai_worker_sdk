# Python binding for Rust AMQP Worker
Based on [rs_amqp_worker](https://github.com/media-cloud-ai/rs_amqp_worker).

[![Build Status](https://travis-ci.org/media-cloud-ai/py_amqp_worker.svg?branch=master)](https://travis-ci.org/media-cloud-ai/py_amqp_worker)
[![Coverage Status](https://coveralls.io/repos/github/media-cloud-ai/py_amqp_worker/badge.svg?branch=master)](https://coveralls.io/github/media-cloud-ai/py_amqp_worker?branch=master)

## Build
To build the rust application
```bash
cargo build
```

## Test
To run the unit tests, you must build the provided worker example (see the Build section above).
```bash
cargo test
```

## Usage

This worker uses Rust FFI to load a C/C++ Shared Object library, and to execute it. The C/C++ worker must implement some functions to be correctly bound:

 * `String get_name()`: to retrieve the worker name
 * `String get_short_description()`: to retrieve a short description of the worker
 * `String get_description()`: to describe the worker purpose
 * `String get_version()`: to retrieve the worker version
 * `Array<Parameter> get_parameters() `: return the list of parameters for this worker
 * `void process(parameters)`: to execute the worker process

For more details, see the provided [worker.py](worker.py) example.

Set the `PYTHON_WORKER_FILENAME` environment variable to specify the path of your Python worker. Otherwise, the `worker.py` file will be loaded by default.
