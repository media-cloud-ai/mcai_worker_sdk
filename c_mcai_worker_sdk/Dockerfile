FROM rust:1.42-stretch as builder

ADD . /src
WORKDIR /src

RUN rustup default && \
    cargo build --verbose --release && \
    cargo install --path .

FROM debian:stretch
COPY --from=builder /usr/local/cargo/bin/c_amqp_worker /usr/bin

RUN apt update && \
    apt install -y libssl1.1 ca-certificates

ENV AMQP_QUEUE=job_c_worker
CMD c_mcai_worker_sdk
