FROM rust:1.72.1 as base


FROM base as build
WORKDIR /app
COPY . .
RUN cargo build --release


FROM base as final
WORKDIR /app
COPY --from=build /app/target/release/headcrab .

ENV ENVIRONMENT="inject a value via the compose/k8s file, or docker run --env or --env_file"
ENV RUST_LOG="inject a value via the compose/k8s file, or docker run --env or --env_file"

ENTRYPOINT ["./headcrab"]
