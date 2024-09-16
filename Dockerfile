# Builing Stage
ARG RUST_VERSION=1.80.1
FROM rust:${RUST_VERSION}-slim-bookworm AS build
WORKDIR /app
COPY LICENSE LICENSE

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    <<EOF
set -e
cargo build --locked --release
cp target/release/bifrost /bifrost
EOF


# Final Stage
FROM debian:bookworm-slim AS final

COPY --from=build /bifrost /app/bifrost

WORKDIR /app

CMD ["/app/bifrost"]
