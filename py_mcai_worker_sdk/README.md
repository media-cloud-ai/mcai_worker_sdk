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

This worker uses the [PyO3 crate](https://github.com/PyO3/pyo3) to load a Python file, and to execute it.
The Python worker must implement some functions to be correctly bound:

 * `get_name() -> str`: to retrieve the worker name
 * `get_short_description() -> str`: to retrieve a short description of the worker
 * `get_description() -> str`: to describe the worker purpose
 * `get_version() -> str`: to retrieve the worker version
 * `init()`: to initialize the worker process (optional)
 * `process(handle_callback, parameters) -> dict`: to execute the worker process and return the job result

If the `media` feature is enabled, the following function must be implemented:
 * `init_process(stream_handler, format_context, parameters)`: to initialize the media worker process
 * `process_frame(job_id, stream_index, frame)`: to process an input audio/video frame
 * `ending_process(parameters)`: to end the media worker process

__NB:__ the `process(handle_callback, parameters)` function is not called when the `media` feature is enabled. 

For more details, see the provided [worker.py](worker.py) and [media_worker.py](media_worker.py) examples.

Set the `PYTHON_WORKER_FILENAME` environment variable to specify the path of your Python worker. Otherwise, the `worker.py` file will be loaded by default.
