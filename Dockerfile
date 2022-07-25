FROM rust:slim-bullseye

RUN apt update && apt install pkg-config libssl-dev ca-certificates  -y

WORKDIR /okto
COPY . .

RUN cargo install --path . && \
  cp /usr/local/cargo/bin/okto /usr/local/bin/okto

CMD ["okto"]
