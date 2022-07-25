FROM rust:slim-bullseye as builder

RUN apt update && apt install pkg-config libssl-dev -y

WORKDIR /okto
COPY . .

RUN cargo install --path .

FROM debian:buster-slim

RUN apt-get update && apt-get install -y ca-certificates libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/cargo/bin/okto /usr/local/bin/okto

CMD ["okto"]
