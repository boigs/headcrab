# Based on https://github.com/LukeMathWalker/cargo-chef
ARG RUST_VERSION
FROM rust:${RUST_VERSION} AS chef 
RUN cargo install cargo-chef --version 0.1.66
WORKDIR /app


FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json


FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin headcrab


FROM debian:12.5-slim AS final
# Run the container as non-root, copied from https://github.com/dotnet/dotnet-docker/blob/main/src/runtime-deps/8.0/bookworm-slim/amd64/Dockerfile
ENV APP_UID=1654
RUN groupadd --gid=$APP_UID app && useradd -l --uid=$APP_UID --gid=$APP_UID --create-home app
USER $APP_UID

WORKDIR /app
COPY --from=builder /app/target/release/headcrab /usr/local/bin
COPY config config
COPY words words

EXPOSE 4000
ENV ENVIRONMENT="inject a value via the compose/k8s file, or docker run --env or --env_file"
ENV RUST_LOG="inject a value via the compose/k8s file, or docker run --env or --env_file"
ENTRYPOINT ["/usr/local/bin/headcrab"]
