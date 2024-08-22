# Builing Stage
ARG RUST_VERSION=1.80.1
FROM rust:${RUST_VERSION}-slim-bullseye AS build
WORKDIR /app

RUN --mount=type=bind,source=src,target=src \
    --mount=type=bind,source=Cargo.toml,target=Cargo.toml \
    --mount=type=bind,source=Cargo.lock,target=Cargo.lock \
    <<EOF
set -e
cargo build --locked --release
cp target/release/bifrost /bifrost
EOF


# Final Stage
FROM debian:bullseye-slim AS final

COPY --from=build /bifrost /app/bifrost

EXPOSE 80
EXPOSE 443

WORKDIR /app

CMD ["/app/bifrost"]
