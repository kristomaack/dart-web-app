# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS build
ARG TRUNK_VERSION=0.22.1
RUN rustup target add wasm32-unknown-unknown \
 && curl -fsSL "https://github.com/trunk-rs/trunk/releases/download/v${TRUNK_VERSION}/trunk-$(uname -m)-unknown-linux-gnu.tar.gz" \
    | tar -xz -C /usr/local/bin trunk
WORKDIR /app
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    --mount=type=cache,target=/root/.cache/trunk \
    cd crates/web && trunk build --release \
 && cd /app && cargo build --release -p dart-server \
 && cp target/release/dart-server /usr/local/bin/dart-server

FROM debian:bookworm-slim
RUN useradd --system --uid 10001 dart
COPY --from=build /usr/local/bin/dart-server /usr/local/bin/dart-server
COPY --from=build /app/crates/web/dist /srv/dist
ENV DIST_DIR=/srv/dist PORT=8020
EXPOSE 8020
USER dart
HEALTHCHECK --interval=30s --timeout=3s CMD ["dart-server", "healthcheck"]
CMD ["dart-server"]
