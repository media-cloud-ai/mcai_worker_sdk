FROM rust:1.39-stretch as builder

ADD . /src
WORKDIR /src

RUN apt-get update && \
    apt-get install -y libpython3.5-dev && \
    rustup default nightly && \
    cargo build --verbose --release && \
    cargo install --path .

FROM debian:stretch
COPY --from=builder /usr/local/cargo/bin/py_amqp_worker /usr/bin

RUN apt update && \
    apt install -y libssl1.1 ca-certificates

ENV AMQP_QUEUE=job_python_worker
CMD py_amqp_worker
