# ============================================
# Stage 1: build the admin UI (Node + pnpm)
# ============================================
FROM node:22-alpine AS frontend-builder

RUN corepack enable && corepack prepare pnpm@12.16.1 --activate

WORKDIR /app/frontend

# pnpm-workspace.yaml carries `allowBuilds` (which packages may run install
# scripts). Without it pnpm 12 refuses to install because esbuild's postinstall
# was never approved.
COPY frontend/package.json frontend/pnpm-lock.yaml frontend/pnpm-workspace.yaml ./
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

# `VOLUME` below creates /app/data as root when Docker materialises it, so the
# directory must exist and be owned by `nsm` before that line runs.
RUN apk add --no-cache ca-certificates tzdata && \
    adduser -D -H -u 10001 -s /sbin/nologin nsm && \
    mkdir -p /app/data && \
    chown -R nsm:nsm /app

COPY --from=rust-builder /app/nsm /usr/local/bin/nsm

WORKDIR /app

EXPOSE 3000

VOLUME ["/app/data"]

USER nsm

# Containers have no TTY, so without this logs would go to /app/data/logs/nsm.log
# and `docker compose logs` would stay empty.
ENV NSM_LOG_STDOUT=true

# A TCP connect instead of an HTTP GET: the admin prefix and the accepted hosts
# are configurable, so no single URL is guaranteed to answer from inside the
# container. A listener on the configured port is the one thing `bind_addr`
# always provides.
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD nc -z 127.0.0.1 3000 || exit 1

ENTRYPOINT ["/usr/local/bin/nsm"]
CMD ["-c", "/app/config.yml"]
