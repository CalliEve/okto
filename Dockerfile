FROM rust:slim-bullseye as builder

RUN apt-get update && apt-get install -y pkg-config libssl-dev && cargo install --locked tokio-console

ENV RUSTFLAGS="--cfg tokio_unstable"
ENV RUST_BACKTRACE=1

WORKDIR /okto
COPY . .

RUN cargo install --path .

CMD ["okto"]
