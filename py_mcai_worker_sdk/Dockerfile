FROM rust:1.47-buster as builder

ADD . /src
WORKDIR /src/py_mcai_worker_sdk

RUN apt-get update && \
    apt-get install -y libpython3.7-dev && \
    rustup default nightly && \
    cargo build --verbose --release && \
    cargo install --path .

FROM debian:buster
COPY --from=builder /usr/local/cargo/bin/py_mcai_worker_sdk /usr/bin

RUN apt update && \
    apt install -y \
        libssl1.1 \
        ca-certificates \
        libpython3.7

ENV AMQP_QUEUE=job_python_worker
CMD py_mcai_worker_sdk
