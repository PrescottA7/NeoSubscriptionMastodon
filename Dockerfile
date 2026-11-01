# ============================================
# Stage 1: build the admin UI (Node + pnpm)
# ============================================
FROM node:22-alpine AS frontend-builder

RUN corepack enable && corepack prepare pnpm@latest --activate

WORKDIR /app/frontend

COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile

COPY frontend/ ./
RUN pnpm run build

# ============================================
# Stage 2: build the Rust binary (musl, static)
# ============================================
FROM rust:1-alpine AS rust-builder

RUN apk add --no-cache musl-dev perl make

WORKDIR /app

COPY Cargo.toml Cargo.lock build.rs ./
COPY src/ ./src/
COPY 404.html ./

# The prebuilt bundle is copied in and build.rs skips the UI build because pnpm
# is not installed in this stage.
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

RUN cargo build --release && \
    cp target/release/nsm /app/nsm

# ============================================
# Stage 3: runtime
# ============================================
FROM alpine:3.21

RUN apk add --no-cache ca-certificates tzdata && \
    adduser -D -H -u 10001 -s /sbin/nologin nsm && \
    mkdir -p /app && \
    chown -R nsm:nsm /app

COPY --from=rust-builder /app/nsm /usr/local/bin/nsm

WORKDIR /app

EXPOSE 3000

VOLUME ["/app/data"]

USER nsm

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD wget -qO- http://127.0.0.1:3000/api/health || exit 1

ENTRYPOINT ["/usr/local/bin/nsm"]
CMD ["-c", "/app/config.yml"]
