FROM rust:1.72.1

WORKDIR /app

COPY Cargo.toml ./
COPY Cargo.lock ./

COPY config config

COPY src src

RUN cargo build -r

ENV ENVIRONMENT="inject a value via the compose/k8s file, or docker run --env or --env_file"

ENTRYPOINT ["./target/release/headcrab"]
