FROM rust:1.72.1 as build

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src src

RUN cargo build -r

ENTRYPOINT ["./target/release/headcrab"]
