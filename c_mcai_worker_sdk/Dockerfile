FROM rust:1.46-buster as builder

ADD . /src
WORKDIR /src/c_mcai_worker_sdk

RUN rustup default && \
    cargo build --verbose --release && \
    cargo install --path .

FROM debian:buster
COPY --from=builder /usr/local/cargo/bin/c_mcai_worker_sdk /usr/bin

RUN apt update && \
    apt install -y libssl1.1 ca-certificates

ENV AMQP_QUEUE=job_c_worker
CMD c_mcai_worker_sdk
