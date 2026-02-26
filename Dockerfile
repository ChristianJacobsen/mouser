FROM dhi.io/rust:1-alpine3.22-sfw-dev AS chef
RUN apk add --no-cache tzdata=2025c-r0 \
    && cargo install cargo-chef --locked
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked

FROM dhi.io/alpine-base:3.22
COPY --from=builder /usr/share/zoneinfo /usr/share/zoneinfo
COPY --from=builder /build/target/release/mouser /usr/local/bin/mouser
EXPOSE 7878
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- http://127.0.0.1:${MOUSER_PORT:-7878}/health || exit 1
ENTRYPOINT ["mouser"]
