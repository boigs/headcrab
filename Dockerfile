# Based on https://github.com/LukeMathWalker/cargo-chef
FROM rust:1.77.1 AS chef 
RUN cargo install cargo-chef --version 0.1.66
WORKDIR /app


FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin headcrab


FROM debian:12.5-slim AS final
WORKDIR /app
COPY --from=builder /app/target/release/headcrab /usr/local/bin
COPY config config

RUN addgroup --system --gid 1001 appgroup
RUN adduser --system --uid 1001 --ingroup appgroup app
USER app

EXPOSE 4000
ENV ENVIRONMENT="inject a value via the compose/k8s file, or docker run --env or --env_file"
ENV RUST_LOG="inject a value via the compose/k8s file, or docker run --env or --env_file"
ENTRYPOINT ["/usr/local/bin/headcrab"]
