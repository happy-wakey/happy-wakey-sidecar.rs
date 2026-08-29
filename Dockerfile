FROM rust:1.88.0-bookworm AS build
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release --locked --bin happy-wakey-sidecar

FROM debian:bookworm-slim
RUN useradd --system --uid 65532 --no-create-home --shell /usr/sbin/nologin sidecar
COPY --from=build /src/target/release/happy-wakey-sidecar /usr/local/bin/happy-wakey-sidecar
USER 65532:65532
EXPOSE 9090
ENTRYPOINT ["/usr/local/bin/happy-wakey-sidecar"]
